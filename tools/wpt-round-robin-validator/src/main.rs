// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::process;

const REQUIRED_SECURITY_SCENARIOS: &[&str] = &[
    "WPT-SEC-AUTH-SPOOF-001",
    "WPT-SEC-CMD-REPLAY-001",
    "WPT-SEC-CMD-STALE-001",
    "WPT-SEC-FW-INTEGRITY-001",
    "WPT-SEC-SENSOR-PHASE-001",
    "WPT-SEC-SENSOR-CURRENT-001",
    "WPT-SEC-TIMING-JITTER-001",
    "WPT-SEC-COMM-DOS-001",
    "WPT-SEC-CERT-REVOKE-001",
    "WPT-SEC-RESONANT-SWITCH-001",
];

const TOP_LEVEL: &[&str] = &[
    "schema_version",
    "run_id",
    "protocol",
    "system",
    "conditions",
    "metrology",
    "energy_balance",
    "power_quality",
    "safety",
    "artifact_refs",
    "resonant_networks",
    "cyber_physical_security",
    "standards_basis",
    "motion_profile",
    "result",
];

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(*key))
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(value, path).and_then(Value::as_str)
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    at(value, path).and_then(Value::as_f64)
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

fn exact_object_keys(
    value: Option<&Value>,
    expected: &[&str],
    path: &str,
    errors: &mut Vec<String>,
) {
    let Some(object) = value.and_then(Value::as_object) else {
        errors.push(format!("{path}: expected object"));
        return;
    };
    for key in expected {
        if !object.contains_key(*key) {
            errors.push(format!("{path}.{key}: required field missing"));
        }
    }
    for key in object.keys() {
        if !expected.contains(&key.as_str()) {
            errors.push(format!("{path}.{key}: unexpected field"));
        }
    }
}

fn exact_array_item_keys(
    value: Option<&Value>,
    expected: &[&str],
    path: &str,
    errors: &mut Vec<String>,
) {
    let Some(items) = value.and_then(Value::as_array) else {
        errors.push(format!("{path}: expected array"));
        return;
    };
    for (index, item) in items.iter().enumerate() {
        exact_object_keys(
            Some(item),
            expected,
            &format!("{path}[{index}]"),
            errors,
        );
    }
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_uppercase() || first.is_ascii_digit())
        && chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || "._-".contains(ch))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// Round-robin timestamps are deliberately normalized to RFC3339 UTC whole
// seconds (YYYY-MM-DDTHH:MM:SSZ), avoiding offset and locale ambiguity.
fn parse_utc_seconds(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }

    let part = |start: usize, end: usize| -> Option<i64> {
        std::str::from_utf8(&bytes[start..end]).ok()?.parse().ok()
    };
    let year = part(0, 4)?;
    let month = part(5, 7)?;
    let day = part(8, 10)?;
    let hour = part(11, 13)?;
    let minute = part(14, 16)?;
    let second = part(17, 19)?;

    if !(1..=12).contains(&month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }

    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=month_days).contains(&day) {
        return None;
    }

    // Days since 1970-01-01, using the proleptic Gregorian calendar.
    let adjusted_year = year - i64::from(month <= 2);
    let era = (if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    }) / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn validate_structure(manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    exact_object_keys(Some(manifest), TOP_LEVEL, "$", &mut errors);
    if string_at(manifest, &["schema_version"]) != Some("wpt-test-run-v1") {
        errors.push("schema_version: expected wpt-test-run-v1".into());
    }
    match string_at(manifest, &["run_id"]) {
        Some(id) if valid_identifier(id) => {}
        _ => errors.push("run_id: expected uppercase stable identifier".into()),
    }

    exact_object_keys(
        manifest.get("protocol"),
        &["protocol_id", "version", "revision"],
        "protocol",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("system"),
        &[
            "system_id",
            "technology",
            "topology",
            "hardware_revision",
            "controller_revision",
            "power_rating_w",
            "configuration_digest",
            "concurrent_resonant_networks_allowed",
        ],
        "system",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("conditions"),
        &[
            "direction",
            "power_class",
            "commanded_power_w",
            "frequency_hz",
            "transfer_gap_m",
            "offset_x_m",
            "offset_y_m",
            "offset_z_m",
            "yaw_deg",
            "receiver_count",
            "active_receiver_count",
            "battery_voltage_v",
            "load_profile_id",
            "ambient_temperature_c",
            "relative_humidity_pct",
            "thermal_state",
            "operating_mode",
            "installation_type",
            "rated_apparent_power_va",
        ],
        "conditions",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("motion_profile"),
        &[
            "profile_id",
            "route_distance_m",
            "elapsed_time_s",
            "speed_mean_mps",
            "speed_min_mps",
            "speed_max_mps",
            "longitudinal_acceleration_abs_max_mps2",
            "transmitter_pitch_m",
            "segment_count_total",
            "segment_count_energized",
            "segment_activation_attempts",
            "segment_missed_activations",
            "false_activation_count",
            "roadway_energy_source_j",
            "vehicle_dc_energy_received_j",
            "battery_energy_gain_j",
            "vehicle_propulsion_energy_used_j",
            "roadway_idle_energy_j",
            "roadway_energy_per_km_wh_km",
            "battery_net_energy_per_km_wh_km",
            "energy_sufficiency_ratio",
            "segment_activation_success_fraction",
            "speed_profile_sha256",
            "segment_event_log_sha256",
        ],
        "motion_profile",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("metrology"),
        &[
            "window_start_utc",
            "window_end_utc",
            "sample_rate_hz",
            "synchronization_error_s",
            "power_basis",
            "meters",
            "measurement_planes",
        ],
        "metrology",
        &mut errors,
    );
    exact_array_item_keys(
        at(manifest, &["metrology", "meters"]),
        &[
            "meter_id",
            "role",
            "model",
            "calibration_reference",
            "calibration_due_utc",
            "bandwidth_hz",
            "phase_error_deg",
            "uncertainty_pct",
            "calibration_valid_from_utc",
        ],
        "metrology.meters",
        &mut errors,
    );
    exact_array_item_keys(
        at(manifest, &["metrology", "measurement_planes"]),
        &[
            "plane_id",
            "role",
            "real_power_w",
            "apparent_power_va",
            "reactive_power_var",
            "voltage_rms_v",
            "current_rms_a",
            "power_factor",
            "uncertainty_pct",
        ],
        "metrology.measurement_planes",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("energy_balance"),
        &[
            "source_energy_j",
            "useful_energy_j",
            "loss_terms",
            "stored_energy_delta_j",
            "residual_energy_j",
            "residual_fraction_pct",
            "expanded_uncertainty_pct",
        ],
        "energy_balance",
        &mut errors,
    );
    exact_array_item_keys(
        at(manifest, &["energy_balance", "loss_terms"]),
        &["stage_id", "energy_j", "evidence_kind"],
        "energy_balance.loss_terms",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("power_quality"),
        &[
            "primary_current_thd_pct",
            "secondary_current_thd_pct",
            "voltage_thd_pct",
            "thd_measurement_bandwidth_hz",
            "thd_window_s",
            "switching_frequency_hz",
            "zvs_observed",
        ],
        "power_quality",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("safety"),
        &[
            "standard_edition",
            "exposure_assessment_status",
            "foreign_object_detection",
            "unauthorized_receiver",
            "communication_loss_response",
            "thermal_maximum_c",
        ],
        "safety",
        &mut errors,
    );
    exact_array_item_keys(
        manifest.get("artifact_refs"),
        &[
            "artifact_type",
            "uri",
            "sha256",
            "media_type",
            "description",
        ],
        "artifact_refs",
        &mut errors,
    );
    exact_array_item_keys(
        manifest.get("resonant_networks"),
        &[
            "network_id",
            "nominal_frequency_hz",
            "commanded_state",
            "isolation_switch_state",
            "current_rms_a",
            "current_peak_a",
            "continuous_current_limit_a",
            "current_peak_limit_a",
            "voltage_rms_v",
            "voltage_peak_v",
            "voltage_limit_v",
            "voltage_peak_limit_v",
            "thermal_maximum_c",
            "thermal_limit_c",
            "core_material_id",
            "core_saturation_curve_ref",
            "core_temperature_c",
            "core_flux_density_peak_t",
            "core_flux_density_limit_t",
            "core_flux_assessment_method",
            "measurement_method",
            "artifact_sha256",
        ],
        "resonant_networks",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("cyber_physical_security"),
        &[
            "communication_mode",
            "authentication_method",
            "mutual_authentication",
            "replay_protection",
            "receiver_identity_bound",
            "firmware_integrity_verified",
            "sensor_plausibility_checks",
            "spoofed_receiver_rejected",
            "replay_message_rejected",
            "sensor_tampering_detected",
            "communication_loss_fallback",
            "p99_message_latency_ms",
            "max_message_jitter_ms",
            "max_message_age_ms",
            "control_deadline_ms",
            "max_accepted_command_age_ms",
            "security_artifact_sha256",
            "freshness_mechanism",
            "security_scenario_results",
        ],
        "cyber_physical_security",
        &mut errors,
    );
    exact_array_item_keys(
        at(manifest, &["cyber_physical_security", "security_scenario_results"]),
        &["scenario_id", "outcome", "evidence_sha256", "notes"],
        "cyber_physical_security.security_scenario_results",
        &mut errors,
    );
    exact_array_item_keys(
        manifest.get("standards_basis"),
        &[
            "document_id",
            "edition",
            "applicability",
            "scope_statement",
            "provisions_tested",
            "compliance_claim",
            "evidence_sha256",
            "issuing_body",
            "certificate_id",
            "notes",
        ],
        "standards_basis",
        &mut errors,
    );
    exact_object_keys(
        manifest.get("result"),
        &["qualification", "deviations", "notes"],
        "result",
        &mut errors,
    );

    match string_at(manifest, &["result", "qualification"]) {
        Some(
            "NOT_RUN"
            | "INCOMPLETE"
            | "QUALIFIED"
            | "NOT_COMPARABLE"
            | "MODEL_ONLY"
            | "REJECTED",
        ) => {}
        _ => errors.push("result.qualification: invalid or missing disposition".into()),
    }
    if !manifest
        .get("result")
        .and_then(|value| value.get("deviations"))
        .is_some_and(Value::is_array)
    {
        errors.push("result.deviations: expected array".into());
    }
    if !nonempty(string_at(manifest, &["result", "notes"])) {
        errors.push("result.notes: nonempty explanation required".into());
    }
    errors
}

fn validate_qualified(manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    for path in [
        &["protocol", "protocol_id"][..],
        &["protocol", "version"][..],
        &["system", "system_id"][..],
        &["system", "topology"][..],
        &["system", "hardware_revision"][..],
        &["system", "controller_revision"][..],
    ] {
        if !nonempty(string_at(manifest, path)) {
            errors.push(format!("{}: required for QUALIFIED", path.join(".")));
        }
    }
    if string_at(manifest, &["protocol", "protocol_id"]) != Some("WPT-RR-01") {
        errors.push("protocol.protocol_id: unsupported protocol".into());
    }
    match string_at(manifest, &["system", "configuration_digest"]) {
        Some(digest) if valid_sha256(digest) => {}
        _ => errors.push("system.configuration_digest: require 64-character SHA-256".into()),
    }
    match number_at(manifest, &["system", "power_rating_w"]) {
        Some(value) if value > 0.0 => {}
        _ => errors.push("system.power_rating_w: positive rating required".into()),
    }


    let technology = string_at(manifest, &["system", "technology"]).unwrap_or("");
    let operating_mode =
        string_at(manifest, &["conditions", "operating_mode"]).unwrap_or("UNKNOWN");
    if operating_mode == "DYNAMIC" {
        let motion = manifest.get("motion_profile").unwrap_or(&Value::Null);
        let distance = number_at(motion, &["route_distance_m"]);
        let elapsed = number_at(motion, &["elapsed_time_s"]);
        let speed_mean = number_at(motion, &["speed_mean_mps"]);
        let speed_min = number_at(motion, &["speed_min_mps"]);
        let speed_max = number_at(motion, &["speed_max_mps"]);
        let accel = number_at(motion, &["longitudinal_acceleration_abs_max_mps2"]);
        let pitch = number_at(motion, &["transmitter_pitch_m"]);
        let total_segments = at(motion, &["segment_count_total"]).and_then(Value::as_u64);
        let energized_segments = at(motion, &["segment_count_energized"]).and_then(Value::as_u64);
        let activation_attempts = at(motion, &["segment_activation_attempts"]).and_then(Value::as_u64);
        let missed = at(motion, &["segment_missed_activations"]).and_then(Value::as_u64);
        let false_activations = at(motion, &["false_activation_count"]).and_then(Value::as_u64);

        for (field, value, positive) in [
            ("route_distance_m", distance, true),
            ("elapsed_time_s", elapsed, true),
            ("speed_mean_mps", speed_mean, false),
            ("speed_min_mps", speed_min, false),
            ("speed_max_mps", speed_max, false),
            ("longitudinal_acceleration_abs_max_mps2", accel, false),
            ("transmitter_pitch_m", pitch, true),
        ] {
            if !value.is_some_and(|v| v.is_finite() && if positive { v > 0.0 } else { v >= 0.0 }) {
                errors.push(format!("motion_profile.{field}: valid positive/nonnegative dynamic measurement required"));
            }
        }
        match (speed_min, speed_mean, speed_max) {
            (Some(min), Some(mean), Some(max)) if min <= mean && mean <= max => {}
            _ => errors.push("motion_profile: require speed_min <= speed_mean <= speed_max".into()),
        }
        match (total_segments, energized_segments, activation_attempts, missed) {
            (Some(total), Some(energized), Some(attempts), Some(missed_count))
                if total > 0 && energized <= total && attempts > 0 && missed_count <= attempts => {}
            _ => errors.push("motion_profile: invalid segment/activation event counts".into()),
        }
        if !false_activations.is_some_and(|count| count == 0) {
            errors.push("motion_profile.false_activation_count: zero false/unauthorized segment activations required".into());
        }
        let source_j = number_at(motion, &["roadway_energy_source_j"]);
        let dc_j = number_at(motion, &["vehicle_dc_energy_received_j"]);
        let battery_j = number_at(motion, &["battery_energy_gain_j"]);
        let propulsion_j = number_at(motion, &["vehicle_propulsion_energy_used_j"]);
        let idle_j = number_at(motion, &["roadway_idle_energy_j"]);
        for (field, value) in [
            ("roadway_energy_source_j", source_j),
            ("vehicle_dc_energy_received_j", dc_j),
            ("battery_energy_gain_j", battery_j),
            ("vehicle_propulsion_energy_used_j", propulsion_j),
            ("roadway_idle_energy_j", idle_j),
        ] {
            if !value.is_some_and(|v| v.is_finite() && v >= 0.0) {
                errors.push(format!("motion_profile.{field}: nonnegative energy required"));
            }
        }
        if let (Some(source), Some(received)) = (source_j, dc_j) {
            if received > source {
                errors.push("motion_profile.vehicle_dc_energy_received_j: cannot exceed roadway source energy".into());
            }
        }
        if let (Some(source), Some(idle)) = (source_j, idle_j) {
            if idle > source {
                errors.push("motion_profile.roadway_idle_energy_j: cannot exceed total roadway source energy".into());
            }
        }
        if let (Some(distance_m), Some(received_j), Some(battery_gain), Some(propulsion), Some(idle)) =
            (distance, dc_j, battery_j, propulsion_j, idle_j)
        {
            let route_km = distance_m / 1000.0;
            let expected_wh_km = received_j / 3600.0 / route_km;
            let expected_net_wh_km = (battery_gain - propulsion) / 3600.0 / route_km;
            let expected_sufficiency = if propulsion > 0.0 { battery_gain / propulsion } else { 0.0 };
            let expected_activation = match (activation_attempts, missed) {
                (Some(attempts), Some(missed_count)) if attempts > 0 =>
                    (attempts - missed_count) as f64 / attempts as f64,
                _ => 0.0,
            };
            let reported = [
                ("roadway_energy_per_km_wh_km", number_at(motion, &["roadway_energy_per_km_wh_km"]), expected_wh_km),
                ("battery_net_energy_per_km_wh_km", number_at(motion, &["battery_net_energy_per_km_wh_km"]), expected_net_wh_km),
                ("energy_sufficiency_ratio", number_at(motion, &["energy_sufficiency_ratio"]), expected_sufficiency),
                ("segment_activation_success_fraction", number_at(motion, &["segment_activation_success_fraction"]), expected_activation),
            ];
            for (field, actual, expected) in reported {
                match actual {
                    Some(value) if (value - expected).abs() <= 1e-6_f64.max(expected.abs() * 1e-4) => {}
                    _ => errors.push(format!(
                        "motion_profile.{field}: reported value does not match route energy/event ledger (expected {expected:.9})"
                    )),
                }
            }
            if propulsion <= 0.0 {
                errors.push("motion_profile.vehicle_propulsion_energy_used_j: positive propulsion energy required to calculate energy sufficiency".into());
            }
            if !source.is_finite() || source <= 0.0 || idle > source {
                errors.push("motion_profile: roadway source/idle energy ledger invalid".into());
            }
        } else {
            errors.push("motion_profile: full source/vehicle/battery/propulsion energy ledger required".into());
        }
        if !string_at(motion, &["profile_id"]).is_some_and(|id| !id.trim().is_empty()) {
            errors.push("motion_profile.profile_id: frozen drive/route profile ID required".into());
        }
        for digest in ["speed_profile_sha256", "segment_event_log_sha256"] {
            if !string_at(motion, &[digest]).is_some_and(valid_sha256) {
                errors.push(format!("motion_profile.{digest}: SHA-256 evidence digest required"));
            }
        }
    } else if operating_mode == "STATIC" {
        let motion = manifest.get("motion_profile").unwrap_or(&Value::Null);
        if number_at(motion, &["route_distance_m"]).is_some()
            || number_at(motion, &["elapsed_time_s"]).is_some()
            || number_at(motion, &["roadway_energy_source_j"]).is_some()
            || number_at(motion, &["vehicle_dc_energy_received_j"]).is_some()
            || number_at(motion, &["energy_sufficiency_ratio"]).is_some()
        {
            errors.push("motion_profile: dynamic route metrics must remain null for STATIC runs".into());
        }
    } else {
        errors.push("conditions.operating_mode: STATIC or DYNAMIC required for QUALIFIED run".into());
    }

    if !matches!(
        string_at(manifest, &["conditions", "direction"]),
        Some("GRID_TO_VEHICLE" | "VEHICLE_TO_GRID" | "UNIDIRECTIONAL")
    ) {
        errors.push("conditions.direction: physical power-flow direction required".into());
    }
    if string_at(manifest, &["conditions", "power_class"]) == Some("NOT_APPLICABLE") {
        errors.push("conditions.power_class: declare a power class".into());
    }
    for (path, predicate) in [
        (&["conditions", "commanded_power_w"][..], "positive"),
        (&["conditions", "frequency_hz"][..], "positive"),
        (&["conditions", "transfer_gap_m"][..], "nonnegative"),
    ] {
        let valid = number_at(manifest, path).is_some_and(|v| {
            if predicate == "positive" {
                v > 0.0
            } else {
                v >= 0.0
            }
        });
        if !valid {
            errors.push(format!("{}: {predicate} value required", path.join(".")));
        }
    }
    let receiver_count = at(manifest, &["conditions", "receiver_count"])
        .and_then(Value::as_u64);
    let active_count = at(manifest, &["conditions", "active_receiver_count"])
        .and_then(Value::as_u64);
    match (receiver_count, active_count) {
        (Some(total), Some(active)) if total > 0 && active > 0 && active <= total => {}
        _ => errors.push(
            "conditions.receiver_count/active_receiver_count: require 0 < active <= total"
                .into(),
        ),
    }
    if string_at(manifest, &["conditions", "thermal_state"]) == Some("UNKNOWN") {
        errors.push("conditions.thermal_state: known thermal state required".into());
    }

    let start_text = string_at(manifest, &["metrology", "window_start_utc"]);
    let end_text = string_at(manifest, &["metrology", "window_end_utc"]);
    let start = start_text.and_then(parse_utc_seconds);
    let end = end_text.and_then(parse_utc_seconds);
    match (start, end) {
        (Some(start), Some(end)) if end > start => {}
        _ => errors.push(
            concat!(
                "metrology.window_start_utc/window_end_utc: require ordered UTC timestamps ",
                "in YYYY-MM-DDTHH:MM:SSZ"
            )
            .into(),
        ),
    }
    if !number_at(manifest, &["metrology", "sample_rate_hz"]).is_some_and(|v| v > 0.0) {
        errors.push("metrology.sample_rate_hz: positive sample rate required".into());
    }
    if !number_at(manifest, &["metrology", "synchronization_error_s"])
        .is_some_and(|v| v >= 0.0)
    {
        errors.push("metrology.synchronization_error_s: nonnegative value required".into());
    }
    if string_at(manifest, &["metrology", "power_basis"]) != Some("REAL_POWER") {
        errors.push("metrology.power_basis: REAL_POWER required for energy accounting".into());
    }

    let meters = manifest
        .get("metrology")
        .and_then(|value| value.get("meters"))
        .and_then(Value::as_array);
    let mut meter_ids = BTreeSet::new();
    let mut has_source_meter = false;
    let mut has_receiver_meter = false;
    match meters {
        Some(meters) if meters.len() >= 2 => {
            for (index, meter) in meters.iter().enumerate() {
                let id = string_at(meter, &["meter_id"]).unwrap_or("");
                if !nonempty(Some(id)) || !meter_ids.insert(id.to_owned()) {
                    errors.push(format!(
                        "metrology.meters[{index}].meter_id: missing or duplicate"
                    ));
                }
                match string_at(meter, &["role"]) {
                    Some("SOURCE" | "PRIMARY") => has_source_meter = true,
                    Some("RECEIVER" | "BATTERY") => has_receiver_meter = true,
                    _ => {}
                }
                for field in ["model", "calibration_reference"] {
                    if !nonempty(string_at(meter, &[field])) {
                        errors.push(format!(
                            "metrology.meters[{index}].{field}: required for QUALIFIED"
                        ));
                    }
                }
                let valid_from = string_at(meter, &["calibration_valid_from_utc"])
                    .and_then(parse_utc_seconds);
                let due = string_at(meter, &["calibration_due_utc"])
                    .and_then(parse_utc_seconds);
                match (valid_from, due, start, end) {
                    (Some(valid_from), Some(due), Some(start), Some(end))
                        if valid_from <= start && due >= end && due >= valid_from => {}
                    _ => errors.push(format!(
                        "metrology.meters[{index}]: calibration must cover the complete run window"
                    )),
                }
                if !number_at(meter, &["uncertainty_pct"]).is_some_and(|v| v >= 0.0) {
                    errors.push(format!(
                        concat!(
                            "metrology.meters[{index}].uncertainty_pct: ",
                            "nonnegative uncertainty required"
                        )
                    ));
                }
                if !number_at(meter, &["bandwidth_hz"]).is_some_and(|v| v > 0.0) {
                    errors.push(format!(
                        "metrology.meters[{index}].bandwidth_hz: positive bandwidth required"
                    ));
                }
            }
        }
        _ => errors.push("metrology.meters: at least two calibrated meters required".into()),
    }
    if !has_source_meter {
        errors.push("metrology.meters: SOURCE or PRIMARY meter required".into());
    }
    if !has_receiver_meter {
        errors.push("metrology.meters: RECEIVER or BATTERY meter required".into());
    }

    let planes = manifest
        .get("metrology")
        .and_then(|value| value.get("measurement_planes"))
        .and_then(Value::as_array);
    let mut plane_ids = BTreeSet::new();
    let mut has_input_plane = false;
    let mut has_output_plane = false;
    match planes {
        Some(planes) if planes.len() >= 2 => {
            for (index, plane) in planes.iter().enumerate() {
                let id = string_at(plane, &["plane_id"]).unwrap_or("");
                if !nonempty(Some(id)) || !plane_ids.insert(id.to_owned()) {
                    errors.push(format!(
                        "metrology.measurement_planes[{index}].plane_id: missing or duplicate"
                    ));
                }
                let role = string_at(plane, &["role"]).unwrap_or("");
                let is_input = matches!(
                    role,
                    "MAINS_AC_INPUT" | "SYSTEM_DC_INPUT" | "PRIMARY_DC_BUS"
                );
                let is_output = matches!(
                    role,
                    "RECEIVER_AC_OUTPUT"
                        | "RECTIFIED_DC_OUTPUT"
                        | "BATTERY_DC_TERMINAL"
                        | "USEFUL_LOAD_DC"
                );
                if is_input {
                    has_input_plane = true;
                    if !number_at(plane, &["real_power_w"]).is_some_and(|v| v > 0.0) {
                        errors.push(format!(
                            concat!(
                                "metrology.measurement_planes[{index}].real_power_w: ",
                                "positive source-side real power required"
                            )
                        ));
                    }
                }
                if is_output {
                    has_output_plane = true;
                }
                if is_output {
                    if !number_at(plane, &["real_power_w"]).is_some_and(|v| v >= 0.0) {
                        errors.push(format!(
                            concat!(
                                "metrology.measurement_planes[{index}].real_power_w: ",
                                "measured real power required"
                            )
                        ));
                    }
                    if !number_at(plane, &["uncertainty_pct"]).is_some_and(|v| v >= 0.0) {
                        errors.push(format!(
                            "metrology.measurement_planes[{index}].uncertainty_pct: required"
                        ));
                    }
                }
            }
        }
        _ => errors.push("metrology.measurement_planes: at least two planes required".into()),
    }
    if !has_input_plane {
        errors.push("metrology.measurement_planes: source-side input plane required".into());
    }
    if !has_output_plane {
        errors.push("metrology.measurement_planes: receiver/useful-output plane required".into());
    }

    let source_energy = number_at(manifest, &["energy_balance", "source_energy_j"]);
    let useful_energy = number_at(manifest, &["energy_balance", "useful_energy_j"]);
    let stored_delta = number_at(manifest, &["energy_balance", "stored_energy_delta_j"]);
    let reported_residual = number_at(manifest, &["energy_balance", "residual_energy_j"]);
    let residual_fraction = number_at(manifest, &["energy_balance", "residual_fraction_pct"]);
    let expanded_uncertainty =
        number_at(manifest, &["energy_balance", "expanded_uncertainty_pct"]);
    let loss_terms = manifest
        .get("energy_balance")
        .and_then(|value| value.get("loss_terms"))
        .and_then(Value::as_array);
    let mut loss_total = 0.0;
    let mut loss_ids = BTreeSet::new();
    match loss_terms {
        Some(terms) => {
            for (index, term) in terms.iter().enumerate() {
                let stage = string_at(term, &["stage_id"]).unwrap_or("");
                if !nonempty(Some(stage)) || !loss_ids.insert(stage.to_owned()) {
                    errors.push(format!(
                        "energy_balance.loss_terms[{index}].stage_id: missing or duplicate"
                    ));
                }
                match number_at(term, &["energy_j"]) {
                    Some(value) if value >= 0.0 => loss_total += value,
                    _ => errors.push(format!(
                        concat!(
                            "energy_balance.loss_terms[{index}].energy_j: ",
                            "measured/calculated nonnegative energy required"
                        )
                    )),
                }
                if !matches!(
                    string_at(term, &["evidence_kind"]),
                    Some("MEASURED" | "AUTHOR_CALCULATED" | "MODEL_DERIVED")
                ) {
                    errors.push(format!(
                        concat!(
                            "energy_balance.loss_terms[{index}].evidence_kind: ",
                            "unknown/inferred losses cannot qualify"
                        )
                    ));
                }
            }
        }
        None => errors.push("energy_balance.loss_terms: array required".into()),
    }

    match (
        source_energy,
        useful_energy,
        stored_delta,
        reported_residual,
        residual_fraction,
        expanded_uncertainty,
    ) {
        (
            Some(source),
            Some(useful),
            Some(stored),
            Some(residual),
            Some(fraction),
            Some(uncertainty),
        ) if source > 0.0 && useful >= 0.0 && uncertainty >= 0.0 => {
            let calculated_residual = source - useful - loss_total - stored;
            let tolerance = (source.abs() * uncertainty / 100.0).max(1e-6);
            if (calculated_residual - residual).abs() > tolerance {
                errors.push(format!(
                    concat!(
                        "energy_balance.residual_energy_j: ledger mismatch; calculated ",
                        "{calculated_residual:.9}, reported {residual:.9} ",
                        "(tolerance {tolerance:.9})"
                    )
                ));
            }
            let calculated_fraction = calculated_residual.abs() / source * 100.0;
            if (calculated_fraction - fraction).abs() > 1e-6 {
                errors.push(format!(
                    concat!(
                        "energy_balance.residual_fraction_pct: expected absolute calculated ",
                        "residual fraction {calculated_fraction:.9}, got {fraction:.9}"
                    )
                ));
            }
            let residual_limit = if string_at(manifest, &["conditions", "thermal_state"])
                == Some("STEADY_STATE")
            {
                0.5
            } else {
                1.0
            };
            if calculated_fraction > residual_limit {
                errors.push(format!(
                    concat!(
                        "energy_balance.residual_fraction_pct: calculated {calculated_fraction:.6}% ",
                        "exceeds proposed {residual_limit:.3}% qualification target"
                    )
                ));
            }
        }
        _ => errors.push(
            concat!(
                "energy_balance: source/useful/stored/residual energies, residual fraction ",
                "and expanded uncertainty must be numeric"
            )
                .into(),
        ),
    }

    let safety_checks = [
        (
            "safety.exposure_assessment_status",
            string_at(manifest, &["safety", "exposure_assessment_status"])
                == Some("ASSESSED"),
        ),
        (
            "safety.foreign_object_detection",
            matches!(
                string_at(manifest, &["safety", "foreign_object_detection"]),
                Some("PASS" | "NOT_APPLICABLE")
            ),
        ),
        (
            "safety.unauthorized_receiver",
            matches!(
                string_at(manifest, &["safety", "unauthorized_receiver"]),
                Some("PASS" | "NOT_APPLICABLE")
            ),
        ),
        (
            "safety.communication_loss_response",
            matches!(
                string_at(manifest, &["safety", "communication_loss_response"]),
                Some("SAFE_DEENERGIZE" | "NOT_APPLICABLE")
            ),
        ),
    ];
    if !nonempty(string_at(manifest, &["safety", "standard_edition"])) {
        errors.push(
            "safety.standard_edition: declare applicable standard/edition or assessment basis".into(),
        );
    }
    for (field, passed) in safety_checks {
        if !passed {
            errors.push(format!("{field}: hard safety gate not satisfied"));
        }
    }
    if !number_at(manifest, &["safety", "thermal_maximum_c"]).is_some() {
        errors.push("safety.thermal_maximum_c: thermal maximum required".into());
    }

    let artifacts = manifest
        .get("artifact_refs")
        .and_then(Value::as_array);
    let mut has_waveform = false;
    let mut has_calibration = false;
    match artifacts {
        Some(artifacts) => {
            for (index, artifact) in artifacts.iter().enumerate() {
                let kind = string_at(artifact, &["artifact_type"]).unwrap_or("");
                has_waveform |= kind == "RAW_WAVEFORM";
                has_calibration |= kind == "CALIBRATION_CERTIFICATE";
                if !nonempty(string_at(artifact, &["uri"])) {
                    errors.push(format!("artifact_refs[{index}].uri: required"));
                }
                if !string_at(artifact, &["sha256"]).is_some_and(valid_sha256) {
                    errors.push(format!("artifact_refs[{index}].sha256: valid SHA-256 required"));
                }
            }
        }
        None => errors.push("artifact_refs: array required".into()),
    }
    if !has_waveform {
        errors.push("artifact_refs: RAW_WAVEFORM artifact required".into());
    }
    if !has_calibration {
        errors.push("artifact_refs: CALIBRATION_CERTIFICATE artifact required".into());
    }



    // Standards applicability is a typed scope decision, not a document-name
    // assertion. In particular J2954_202408 does not cover reverse flow,
    // dynamic installations or power above its published 11 kVA scope.
    let standard_basis = manifest
        .get("standards_basis")
        .and_then(Value::as_array);
    match standard_basis {
        Some(records) if !records.is_empty() => {
            let mut seen_documents = BTreeSet::new();
            for (index, record) in records.iter().enumerate() {
                let document_id = string_at(record, &["document_id"]).unwrap_or("");
                let edition = string_at(record, &["edition"]).unwrap_or("");
                let applicability =
                    string_at(record, &["applicability"]).unwrap_or("OUT_OF_SCOPE_REFERENCE");
                let claim =
                    string_at(record, &["compliance_claim"]).unwrap_or("NO_CLAIM");
                if !nonempty(Some(document_id)) || !nonempty(Some(edition))
                    || !seen_documents.insert((document_id.to_owned(), edition.to_owned()))
                {
                    errors.push(format!(
                        "standards_basis[{index}]: document ID/edition missing or duplicate"
                    ));
                }
                if !nonempty(string_at(record, &["scope_statement"]))
                    || !nonempty(string_at(record, &["notes"]))
                {
                    errors.push(format!(
                        "standards_basis[{index}]: scope statement and notes are required"
                    ));
                }
                if !matches!(
                    applicability,
                    "NORMATIVE_REQUIREMENT"
                        | "PARTIAL_REFERENCE"
                        | "TEST_METHOD_REFERENCE"
                        | "INFORMATION_REPORT"
                        | "OUT_OF_SCOPE_REFERENCE"
                        | "PLANNED_NOT_ISSUED"
                ) {
                    errors.push(format!(
                        "standards_basis[{index}].applicability: unsupported classification"
                    ));
                }
                if !matches!(claim, "NO_CLAIM" | "SELF_ASSESSED" | "INDEPENDENTLY_CERTIFIED") {
                    errors.push(format!(
                        "standards_basis[{index}].compliance_claim: unsupported claim level"
                    ));
                }
                match claim {
                    "NO_CLAIM" => {}
                    "SELF_ASSESSED" => {
                        if !string_at(record, &["evidence_sha256"]).is_some_and(valid_sha256) {
                            errors.push(format!(
                                "standards_basis[{index}]: self-assessment requires a SHA-256 evidence digest"
                            ));
                        }
                    }
                    "INDEPENDENTLY_CERTIFIED" => {
                        if !nonempty(string_at(record, &["issuing_body"]))
                            || !nonempty(string_at(record, &["certificate_id"]))
                        {
                            errors.push(format!(
                                "standards_basis[{index}]: independent certification requires issuing body and certificate ID"
                            ));
                        }
                        if !string_at(record, &["evidence_sha256"]).is_some_and(valid_sha256) {
                            errors.push(format!(
                                "standards_basis[{index}]: independent certification requires evidence digest"
                            ));
                        }
                        let digest = string_at(record, &["evidence_sha256"]);
                        let bound = manifest
                            .get("artifact_refs")
                            .and_then(Value::as_array)
                            .is_some_and(|artifacts| {
                                artifacts.iter().any(|artifact| {
                                    string_at(artifact, &["artifact_type"]) == Some("CERTIFICATION_EVIDENCE")
                                        && string_at(artifact, &["sha256"]) == digest
                                })
                            });
                        if !bound {
                            errors.push(format!(
                                "standards_basis[{index}]: certification digest must bind CERTIFICATION_EVIDENCE artifact"
                            ));
                        }
                    }
                    _ => {}
                }

                if document_id == "SAE J2954" && edition == "J2954_202408" {
                    let direction = string_at(manifest, &["conditions", "direction"]);
                    let operating_mode =
                        string_at(manifest, &["conditions", "operating_mode"]);
                    let installation =
                        string_at(manifest, &["conditions", "installation_type"]);
                    let apparent_power =
                        number_at(manifest, &["conditions", "rated_apparent_power_va"]);
                    if applicability == "NORMATIVE_REQUIREMENT" {
                        if direction != Some("GRID_TO_VEHICLE") {
                            errors.push(format!(
                                "standards_basis[{index}]: SAE J2954_202408 normative use requires GRID_TO_VEHICLE"
                            ));
                        }
                        if operating_mode != Some("STATIC") {
                            errors.push(format!(
                                "standards_basis[{index}]: SAE J2954_202408 normative use requires STATIC operation"
                            ));
                        }
                        if installation != Some("SURFACE_MOUNTED") {
                            errors.push(format!(
                                "standards_basis[{index}]: SAE J2954_202408 normative use requires SURFACE_MOUNTED installation"
                            ));
                        }
                        if !apparent_power.is_some_and(|value| value > 0.0 && value <= 11_000.0) {
                            errors.push(format!(
                                "standards_basis[{index}]: SAE J2954_202408 normative use requires measured/declared apparent power within its published <=11 kVA scope"
                            ));
                        }
                    }
                }
                if document_id == "SAE J2954/2" && applicability == "NORMATIVE_REQUIREMENT" {
                    errors.push(format!(
                        "standards_basis[{index}]: SAE J2954/2_202212 is an Information Report with incomplete detailed requirements, not a complete normative compliance specification"
                    ));
                }
                if document_id == "SAE J2954/3" && applicability != "PLANNED_NOT_ISSUED" {
                    errors.push(format!(
                        "standards_basis[{index}]: J2954/3 is tracked as planned/unissued in this benchmark revision"
                    ));
                }
            }
        }
        _ => errors.push(
            "standards_basis: qualified run requires an explicit versioned applicability record".into()
        ),
    }

    // Resonant-network branches are first-class electrical ports. A network
    // commanded INACTIVE can still carry induced circulating current.
    if matches!(technology, "IPT" | "CPT" | "HYBRID") {
        let networks = manifest
            .get("resonant_networks")
            .and_then(Value::as_array);
        match networks {
            Some(networks) if !networks.is_empty() => {
                let mut ids = BTreeSet::new();
                let mut active_count = 0usize;
                let concurrent_allowed = at(
                    manifest,
                    &["system", "concurrent_resonant_networks_allowed"],
                )
                .and_then(Value::as_bool);

                if networks.len() > 1 && concurrent_allowed.is_none() {
                    errors.push(
                        "system.concurrent_resonant_networks_allowed: declare whether multiple networks may be active"
                            .into(),
                    );
                }
                for (index, network) in networks.iter().enumerate() {
                    let network_id = string_at(network, &["network_id"]).unwrap_or("");
                    if !nonempty(Some(network_id)) || !ids.insert(network_id.to_owned()) {
                        errors.push(format!(
                            "resonant_networks[{index}].network_id: missing or duplicate"
                        ));
                    }
                    if !number_at(network, &["nominal_frequency_hz"])
                        .is_some_and(|value| value > 0.0)
                    {
                        errors.push(format!(
                            "resonant_networks[{index}].nominal_frequency_hz: positive frequency required"
                        ));
                    }
                    let state = string_at(network, &["commanded_state"]).unwrap_or("UNKNOWN");
                    let switch_state =
                        string_at(network, &["isolation_switch_state"]).unwrap_or("UNKNOWN");
                    let method =
                        string_at(network, &["measurement_method"]).unwrap_or("NOT_REPORTED");
                    if switch_state == "FAULT" {
                        errors.push(format!(
                            "resonant_networks[{index}].isolation_switch_state: faulted switch state"
                        ));
                    }
                    if state == "TRANSITIONING" || state == "UNKNOWN" {
                        errors.push(format!(
                            "resonant_networks[{index}].commanded_state: steady-state run cannot contain transitioning/unknown network"
                        ));
                    }
                    if switch_state == "UNKNOWN" {
                        errors.push(format!(
                            "resonant_networks[{index}].isolation_switch_state: switch state must be known"
                        ));
                    }
                    let current = number_at(network, &["current_rms_a"]);
                    let current_limit =
                        number_at(network, &["continuous_current_limit_a"]);
                    let current_peak = number_at(network, &["current_peak_a"]);
                    let current_peak_limit =
                        number_at(network, &["current_peak_limit_a"]);
                    let voltage = number_at(network, &["voltage_rms_v"]);
                    let voltage_limit = number_at(network, &["voltage_limit_v"]);
                    let voltage_peak = number_at(network, &["voltage_peak_v"]);
                    let voltage_peak_limit =
                        number_at(network, &["voltage_peak_limit_v"]);
                    let temperature = number_at(network, &["thermal_maximum_c"]);
                    let temperature_limit = number_at(network, &["thermal_limit_c"]);
                    match (temperature, temperature_limit) {
                        (Some(value), Some(limit)) if value <= limit => {}
                        _ => errors.push(format!(
                            "resonant_networks[{index}]: temperature missing or exceeds branch thermal limit"
                        )),
                    }
                    let core_method =
                        string_at(network, &["core_flux_assessment_method"]).unwrap_or("NOT_REPORTED");
                    let core_material = string_at(network, &["core_material_id"]);
                    let core_curve = string_at(network, &["core_saturation_curve_ref"]);
                    let core_temperature = number_at(network, &["core_temperature_c"]);
                    let core_flux = number_at(network, &["core_flux_density_peak_t"]);
                    let core_saturation =
                        number_at(network, &["core_flux_density_limit_t"]);
                    match core_method {
                        "MEASURED" | "CALCULATED_FROM_MEASURED_WAVEFORM" => {
                            if !nonempty(core_material) || !nonempty(core_curve) {
                                errors.push(format!(
                                    "resonant_networks[{index}]: core material and saturation curve reference required"
                                ));
                            }
                            match (core_temperature, temperature) {
                                (Some(core_temp), Some(max_temp)) if core_temp <= max_temp => {}
                                _ => errors.push(format!(
                                    "resonant_networks[{index}]: core temperature missing or above declared branch maximum"
                                )),
                            }
                            match (core_flux, core_saturation) {
                                (Some(peak), Some(limit)) if limit > 0.0 => {
                                    let utilization = peak / limit * 100.0;
                                    if utilization > 80.0 {
                                        errors.push(format!(
                                            "resonant_networks[{index}]: core flux utilization {utilization:.3}% exceeds proposed hot-state 80.000% limit"
                                        ));
                                    }
                                }
                                _ => errors.push(format!(
                                    "resonant_networks[{index}]: peak core flux and temperature-specific saturation limit required"
                                )),
                            }
                        }
                        "NOT_APPLICABLE" => {
                            if core_material.is_some()
                                || core_curve.is_some()
                                || core_temperature.is_some()
                                || core_flux.is_some()
                                || core_saturation.is_some()
                            {
                                errors.push(format!(
                                    "resonant_networks[{index}]: NOT_APPLICABLE core method must not carry core material/flux values"
                                ));
                            }
                        }
                        _ => errors.push(format!(
                            "resonant_networks[{index}].core_flux_assessment_method: physical qualification requires measured flux or calculation from measured waveform"
                        )),
                    }
                    if method != "MEASURED" {
                        errors.push(format!(
                            "resonant_networks[{index}].measurement_method: physical branch qualification requires MEASURED evidence"
                        ));
                    }
                    if !string_at(network, &["artifact_sha256"]).is_some_and(valid_sha256) {
                        errors.push(format!(
                            "resonant_networks[{index}].artifact_sha256: waveform/evidence digest required"
                        ));
                    }

                    if state == "ACTIVE" {
                        active_count += 1;
                        match (current, current_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: active branch current missing or exceeds continuous rating"
                            )),
                        }
                        match (current_peak, current_peak_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: active branch peak current missing or exceeds peak rating"
                            )),
                        }
                        match (voltage, voltage_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: active branch voltage missing or exceeds rated voltage"
                            )),
                        }
                        match (voltage_peak, voltage_peak_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: active branch peak voltage missing or exceeds peak rating"
                            )),
                        }
                    } else if state == "INACTIVE" || state == "ISOLATED" {
                        match (current, current_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 => {
                                let leakage_pct = value / limit * 100.0;
                                if leakage_pct > 1.0 {
                                    errors.push(format!(
                                        "resonant_networks[{index}]: inactive branch current {leakage_pct:.3}% of continuous rating exceeds proposed 1.000% target"
                                    ));
                                }
                            }
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: inactive branch current and continuous rating are required"
                            )),
                        }
                        match (current_peak, current_peak_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: inactive branch peak current missing or exceeds peak rating"
                            )),
                        }
                        match (voltage, voltage_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: inactive branch voltage missing or exceeds rated voltage"
                            )),
                        }
                        match (voltage_peak, voltage_peak_limit) {
                            (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                            _ => errors.push(format!(
                                "resonant_networks[{index}]: inactive branch peak voltage missing or exceeds peak rating"
                            )),
                        }
                    } else {
                        errors.push(format!(
                            "resonant_networks[{index}].commanded_state: unsupported steady-state value"
                        ));
                    }
                }
                if active_count == 0 {
                    errors.push("resonant_networks: at least one active branch required".into());
                }
                if concurrent_allowed == Some(false) && active_count != 1 {
                    errors.push(
                        "resonant_networks: exactly one active branch required when concurrent activation is disallowed"
                            .into(),
                    );
                }
            }
            _ => errors.push(
                "resonant_networks: at least one measured branch record required for IPT/CPT/hybrid qualification"
                    .into(),
            ),
        }
    }


    // WPT control is cyber-physical: authenticated commands and trusted sensors
    // must not be allowed to turn a network fault into unintended energy flow.
    let security = manifest
        .get("cyber_physical_security")
        .unwrap_or(&Value::Null);
    let communication_mode =
        string_at(security, &["communication_mode"]).unwrap_or("UNKNOWN");
    let authentication_method =
        string_at(security, &["authentication_method"]).unwrap_or("UNKNOWN");
    if !matches!(communication_mode, "NETWORKED" | "LOCAL_ONLY" | "NONE") {
        errors.push("cyber_physical_security.communication_mode: known mode required".into());
    }
    if !at(manifest, &["cyber_physical_security", "receiver_identity_bound"])
        .and_then(Value::as_bool).unwrap_or(false)
    {
        errors.push("cyber_physical_security.receiver_identity_bound: receiver identity binding required".into());
    }
    if !at(manifest, &["cyber_physical_security", "firmware_integrity_verified"])
        .and_then(Value::as_bool).unwrap_or(false)
    {
        errors.push("cyber_physical_security.firmware_integrity_verified: verified firmware integrity required".into());
    }
    if !at(manifest, &["cyber_physical_security", "sensor_plausibility_checks"])
        .and_then(Value::as_bool).unwrap_or(false)
    {
        errors.push("cyber_physical_security.sensor_plausibility_checks: trusted sensor cross-checks required".into());
    }
    if string_at(security, &["spoofed_receiver_rejected"]) != Some("PASS") {
        errors.push("cyber_physical_security.spoofed_receiver_rejected: spoofed receiver must be rejected".into());
    }
    if string_at(security, &["sensor_tampering_detected"]) != Some("PASS") {
        errors.push("cyber_physical_security.sensor_tampering_detected: sensor tampering must be detected".into());
    }
    if !matches!(
        string_at(security, &["communication_loss_fallback"]),
        Some("SAFE_DEENERGIZE" | "LOCAL_SAFE_CONTROL")
    ) {
        errors.push("cyber_physical_security.communication_loss_fallback: safe fallback required".into());
    }
    let security_digest = string_at(security, &["security_artifact_sha256"]);
    if !security_digest.is_some_and(valid_sha256) {
        errors.push("cyber_physical_security.security_artifact_sha256: attack-test evidence digest required".into());
    }

    let scenario_results = at(
        manifest,
        &["cyber_physical_security", "security_scenario_results"],
    )
    .and_then(Value::as_array);
    match scenario_results {
        Some(results) => {
            let mut seen = BTreeSet::new();
            for (index, result) in results.iter().enumerate() {
                let id = string_at(result, &["scenario_id"]).unwrap_or("");
                if !REQUIRED_SECURITY_SCENARIOS.contains(&id) || !seen.insert(id.to_owned()) {
                    errors.push(format!(
                        "cyber_physical_security.security_scenario_results[{index}].scenario_id: unknown or duplicate scenario"
                    ));
                }
                let outcome = string_at(result, &["outcome"]).unwrap_or("NOT_RUN");
                if !matches!(outcome, "PASS" | "NOT_APPLICABLE") {
                    errors.push(format!(
                        "cyber_physical_security.security_scenario_results[{index}].outcome: qualified run must PASS or justify NOT_APPLICABLE"
                    ));
                }
                if outcome == "PASS"
                    && !string_at(result, &["evidence_sha256"]).is_some_and(valid_sha256)
                {
                    errors.push(format!(
                        "cyber_physical_security.security_scenario_results[{index}].evidence_sha256: passing scenario requires evidence digest"
                    ));
                }
                if outcome == "NOT_APPLICABLE"
                    && !nonempty(string_at(result, &["notes"]))
                {
                    errors.push(format!(
                        "cyber_physical_security.security_scenario_results[{index}].notes: NOT_APPLICABLE requires reason"
                    ));
                }
            }
            for id in REQUIRED_SECURITY_SCENARIOS {
                if !seen.contains(*id) {
                    errors.push(format!(
                        "cyber_physical_security.security_scenario_results: missing required scenario {id}"
                    ));
                }
            }
            if results.len() != REQUIRED_SECURITY_SCENARIOS.len() {
                errors.push(format!(
                    "cyber_physical_security.security_scenario_results: expected exactly {} unique scenarios",
                    REQUIRED_SECURITY_SCENARIOS.len()
                ));
            }
        }
        None => errors.push(
            "cyber_physical_security.security_scenario_results: scenario result array required".into()
        ),
    }

    let security_report_bound = manifest
        .get("artifact_refs")
        .and_then(Value::as_array)
        .is_some_and(|artifacts| {
            artifacts.iter().any(|artifact| {
                string_at(artifact, &["artifact_type"]) == Some("SECURITY_REPORT")
                    && string_at(artifact, &["sha256"]) == security_digest
            })
        });
    if !security_report_bound {
        errors.push("cyber_physical_security.security_artifact_sha256: digest must bind a SECURITY_REPORT artifact".into());
    }
    if let Some(results) = at(
        manifest,
        &["cyber_physical_security", "security_scenario_results"],
    )
    .and_then(Value::as_array)
    {
        for (index, result) in results.iter().enumerate() {
            if string_at(result, &["outcome"]) == Some("PASS")
                && string_at(result, &["evidence_sha256"]) != security_digest
            {
                errors.push(format!(
                    "cyber_physical_security.security_scenario_results[{index}].evidence_sha256: PASS evidence must bind the run SECURITY_REPORT digest"
                ));
            }
        }
    }

    match communication_mode {
        "NETWORKED" => {
            if !matches!(authentication_method, "PKI_MUTUAL" | "TLS_MUTUAL") {
                errors.push("cyber_physical_security.authentication_method: mutual PKI/TLS required for networked control".into());
            }
            if !at(manifest, &["cyber_physical_security", "mutual_authentication"])
                .and_then(Value::as_bool).unwrap_or(false)
            {
                errors.push("cyber_physical_security.mutual_authentication: networked session must authenticate both peers".into());
            }
            if !at(manifest, &["cyber_physical_security", "replay_protection"])
                .and_then(Value::as_bool).unwrap_or(false)
            {
                errors.push("cyber_physical_security.replay_protection: fresh nonce/timestamp/sequence required".into());
            }
            if !matches!(
                string_at(security, &["freshness_mechanism"]),
                Some("NONCE" | "TIMESTAMP" | "MONOTONIC_SEQUENCE" | "COMBINED")
            ) {
                errors.push("cyber_physical_security.freshness_mechanism: replay-safe freshness mechanism required".into());
            }
            if string_at(security, &["replay_message_rejected"]) != Some("PASS") {
                errors.push("cyber_physical_security.replay_message_rejected: replay control test must pass".into());
            }
            let latency = number_at(manifest, &["cyber_physical_security", "p99_message_latency_ms"]);
            let deadline = number_at(manifest, &["cyber_physical_security", "control_deadline_ms"]);
            let jitter = number_at(manifest, &["cyber_physical_security", "max_message_jitter_ms"]);
            let age = number_at(manifest, &["cyber_physical_security", "max_message_age_ms"]);
            let max_age = number_at(manifest, &["cyber_physical_security", "max_accepted_command_age_ms"]);
            match (latency, deadline) {
                (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                _ => errors.push("cyber_physical_security.p99_message_latency_ms: measured p99 latency must meet positive control deadline".into()),
            }
            if !jitter.is_some_and(|value| value >= 0.0) {
                errors.push("cyber_physical_security.max_message_jitter_ms: measured jitter required".into());
            }
            match (age, max_age) {
                (Some(value), Some(limit)) if limit > 0.0 && value <= limit => {}
                _ => errors.push("cyber_physical_security.max_message_age_ms: command freshness must meet accepted age bound".into()),
            }
        }
        "LOCAL_ONLY" => {
            if !matches!(
                string_at(security, &["freshness_mechanism"]),
                Some("NOT_APPLICABLE" | "UNKNOWN")
            ) {
                errors.push("cyber_physical_security.freshness_mechanism: declare local-only freshness applicability".into());
            }
            if !matches!(authentication_method, "LOCAL_AUTH" | "NOT_APPLICABLE") {
                errors.push("cyber_physical_security.authentication_method: LOCAL_AUTH or NOT_APPLICABLE required for local-only control".into());
            }
            if !matches!(
                string_at(security, &["replay_message_rejected"]),
                Some("PASS" | "NOT_APPLICABLE")
            ) {
                errors.push("cyber_physical_security.replay_message_rejected: declare local replay test disposition".into());
            }
        }
        "NONE" => {
            if string_at(security, &["freshness_mechanism"]) != Some("NOT_APPLICABLE") {
                errors.push("cyber_physical_security.freshness_mechanism: NOT_APPLICABLE required with no communications".into());
            }
            if authentication_method != "NOT_APPLICABLE" {
                errors.push("cyber_physical_security.authentication_method: NOT_APPLICABLE required with no communications".into());
            }
            if !matches!(
                string_at(security, &["replay_message_rejected"]),
                Some("NOT_APPLICABLE")
            ) {
                errors.push("cyber_physical_security.replay_message_rejected: NOT_APPLICABLE required with no communications".into());
            }
        }
        _ => {}
    }

    if matches!(technology, "IPT" | "CPT" | "HYBRID") {
        if !number_at(manifest, &["power_quality", "primary_current_thd_pct"])
            .is_some_and(|value| value >= 0.0)
        {
            errors.push(
                "power_quality.primary_current_thd_pct: required for inductive/capacitive/hybrid WPT"
                    .into(),
            );
        }
        if !number_at(manifest, &["power_quality", "thd_measurement_bandwidth_hz"])
            .is_some_and(|value| value > 0.0)
            || !number_at(manifest, &["power_quality", "thd_window_s"])
                .is_some_and(|value| value > 0.0)
        {
            errors.push("power_quality: THD bandwidth and analysis window required".into());
        }
    }

    errors
}

pub fn validate_manifest(manifest: &Value) -> (String, Vec<String>) {
    let mut errors = validate_structure(manifest);
    let disposition = string_at(manifest, &["result", "qualification"])
        .unwrap_or("INVALID")
        .to_owned();
    if errors.is_empty() && disposition == "QUALIFIED" {
        errors.extend(validate_qualified(manifest));
    }
    (disposition, errors)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "wpt-round-robin-validator".into());
    let Some(path) = args.next() else {
        eprintln!("usage: {program} <test-run-manifest.json>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("error: expected exactly one manifest path");
        process::exit(2);
    }

    let contents = fs::read_to_string(&path)?;
    let manifest: Value = serde_json::from_str(&contents)?;
    let (disposition, errors) = validate_manifest(&manifest);
    if errors.is_empty() {
        println!("WPT run manifest valid; disposition={disposition}.");
        if disposition == "NOT_RUN" {
            println!("Template/record only: no physical measurement or qualification is implied.");
        }
        return Ok(());
    }

    eprintln!("WPT run manifest rejected ({}) issue(s):", errors.len());
    for error in &errors {
        eprintln!(" - {error}");
    }
    process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn template() -> Value {
        serde_json::from_str(include_str!(
            "../../../benchmarks/wpt-empirical/v1/test-run.example.json"
        ))
        .expect("template JSON parses")
    }

    fn qualified_fixture() -> Value {
        let mut manifest = template();
        manifest["run_id"] = json!("SYNTHETIC-KNOWN-TRUTH-001");
        manifest["system"]["power_rating_w"] = json!(11_000.0);
        manifest["system"]["topology"] = json!("synthetic IPT reference");
        manifest["system"]["hardware_revision"] = json!("rev-A");
        manifest["system"]["controller_revision"] = json!("controller-test-commit");
        manifest["system"]["configuration_digest"] = json!("a".repeat(64));
        manifest["system"]["concurrent_resonant_networks_allowed"] = json!(false);
        manifest["conditions"]["operating_mode"] = json!("STATIC");
        manifest["conditions"]["installation_type"] = json!("SURFACE_MOUNTED");
        manifest["motion_profile"]["profile_id"] = json!("STATIC-KNOWN-TRUTH-NO-ROUTE");

        manifest["conditions"]["rated_apparent_power_va"] = json!(12_100.0);
        manifest["standards_basis"] = json!([
            {
                "document_id": "SAE J2954",
                "edition": "J2954_202408",
                "applicability": "OUT_OF_SCOPE_REFERENCE",
                "scope_statement": "Synthetic 11 kW class reference; AC apparent power 12.1 kVA exceeds the 11 kVA light-duty scope and the fixture is used for bidirectional methodology.",
                "provisions_tested": [],
                "compliance_claim": "NO_CLAIM",
                "evidence_sha256": null,
                "issuing_body": null,
                "certificate_id": null,
                "notes": "Synthetic unit-test context only; not a conformity claim."
            }
        ]);

        manifest["conditions"]["direction"] = json!("GRID_TO_VEHICLE");
        manifest["conditions"]["power_class"] = json!("11_KW");
        manifest["conditions"]["commanded_power_w"] = json!(11_000.0);
        manifest["conditions"]["frequency_hz"] = json!(85_000.0);
        manifest["conditions"]["transfer_gap_m"] = json!(0.16);
        manifest["conditions"]["receiver_count"] = json!(1);
        manifest["conditions"]["active_receiver_count"] = json!(1);
        manifest["conditions"]["battery_voltage_v"] = json!(400.0);
        manifest["conditions"]["thermal_state"] = json!("STEADY_STATE");
        manifest["conditions"]["ambient_temperature_c"] = json!(23.0);
        manifest["conditions"]["relative_humidity_pct"] = json!(45.0);

        manifest["metrology"]["window_start_utc"] = json!("2026-10-01T10:00:00Z");
        manifest["metrology"]["window_end_utc"] = json!("2026-10-01T11:00:00Z");
        manifest["metrology"]["sample_rate_hz"] = json!(1_000_000.0);
        manifest["metrology"]["synchronization_error_s"] = json!(0.000001);
        manifest["metrology"]["power_basis"] = json!("REAL_POWER");
        for meter in manifest["metrology"]["meters"].as_array_mut().unwrap() {
            meter["model"] = json!("synthetic traceable power analyser");
            meter["calibration_reference"] = json!("CAL-SYNTHETIC-001");
            meter["calibration_valid_from_utc"] = json!("2026-01-01T00:00:00Z");
            meter["calibration_due_utc"] = json!("2027-01-01T00:00:00Z");
            meter["bandwidth_hz"] = json!(2_000_000.0);
            meter["phase_error_deg"] = json!(0.01);
            meter["uncertainty_pct"] = json!(0.2);
        }
        manifest["metrology"]["meters"][0]["role"] = json!("SOURCE");
        manifest["metrology"]["meters"][1]["role"] = json!("BATTERY");

        let planes = manifest["metrology"]["measurement_planes"]
            .as_array_mut()
            .unwrap();
        planes[0]["role"] = json!("MAINS_AC_INPUT");
        planes[0]["real_power_w"] = json!(12_000.0);
        planes[0]["apparent_power_va"] = json!(12_100.0);
        planes[0]["reactive_power_var"] = json!(1_552.0);
        planes[0]["voltage_rms_v"] = json!(230.0);
        planes[0]["current_rms_a"] = json!(52.6);
        planes[0]["power_factor"] = json!(0.992);
        planes[0]["uncertainty_pct"] = json!(0.2);
        planes[1]["role"] = json!("BATTERY_DC_TERMINAL");
        planes[1]["real_power_w"] = json!(11_000.0);
        planes[1]["voltage_rms_v"] = json!(400.0);
        planes[1]["current_rms_a"] = json!(27.5);
        planes[1]["uncertainty_pct"] = json!(0.2);

        manifest["energy_balance"]["source_energy_j"] = json!(43_200_000.0);
        manifest["energy_balance"]["useful_energy_j"] = json!(39_600_000.0);
        manifest["energy_balance"]["loss_terms"] = json!([
            {"stage_id":"conversion-losses","energy_j":3_500_000.0,"evidence_kind":"MEASURED"}
        ]);
        manifest["energy_balance"]["stored_energy_delta_j"] = json!(0.0);
        manifest["energy_balance"]["residual_energy_j"] = json!(100_000.0);
        manifest["energy_balance"]["residual_fraction_pct"] =
            json!(100_000.0 / 43_200_000.0 * 100.0);
        manifest["energy_balance"]["expanded_uncertainty_pct"] = json!(0.5);

        manifest["power_quality"]["primary_current_thd_pct"] = json!(4.4);
        manifest["power_quality"]["secondary_current_thd_pct"] = json!(3.2);
        manifest["power_quality"]["voltage_thd_pct"] = json!(1.5);
        manifest["power_quality"]["thd_measurement_bandwidth_hz"] = json!(1_000_000.0);
        manifest["power_quality"]["thd_window_s"] = json!(1.0);
        manifest["power_quality"]["switching_frequency_hz"] = json!(85_000.0);
        manifest["power_quality"]["zvs_observed"] = json!(true);
        manifest["resonant_networks"] = json!([
            {
                "network_id": "RES-85KHZ",
                "nominal_frequency_hz": 85_000.0,
                "commanded_state": "ACTIVE",
                "isolation_switch_state": "CLOSED",
                "current_rms_a": 22.0,
                "current_peak_a": 31.1,
                "continuous_current_limit_a": 31.4,
                "current_peak_limit_a": 44.4,
                "voltage_rms_v": 724.0,
                "voltage_peak_v": 1_024.0,
                "voltage_limit_v": 1_000.0,
                "voltage_peak_limit_v": 1_400.0,
                "thermal_maximum_c": 64.0,
                "thermal_limit_c": 100.0,
                "core_material_id": "synthetic MnZn ferrite",
                "core_saturation_curve_ref": "synthetic Bsat(T) test curve",
                "core_temperature_c": 64.0,
                "core_flux_density_peak_t": 0.30,
                "core_flux_density_limit_t": 0.45,
                "core_flux_assessment_method": "CALCULATED_FROM_MEASURED_WAVEFORM",
                "measurement_method": "MEASURED",
                "artifact_sha256": "c".repeat(64)
            },
            {
                "network_id": "RES-20KHZ",
                "nominal_frequency_hz": 20_000.0,
                "commanded_state": "INACTIVE",
                "isolation_switch_state": "OPEN",
                "current_rms_a": 0.2,
                "current_peak_a": 0.3,
                "continuous_current_limit_a": 94.2,
                "current_peak_limit_a": 133.2,
                "voltage_rms_v": 10.0,
                "voltage_peak_v": 14.2,
                "voltage_limit_v": 2_400.0,
                "voltage_peak_limit_v": 3_400.0,
                "thermal_maximum_c": 35.0,
                "thermal_limit_c": 100.0,
                "core_material_id": "synthetic MnZn ferrite",
                "core_saturation_curve_ref": "synthetic Bsat(T) test curve",
                "core_temperature_c": 35.0,
                "core_flux_density_peak_t": 0.10,
                "core_flux_density_limit_t": 0.45,
                "core_flux_assessment_method": "CALCULATED_FROM_MEASURED_WAVEFORM",
                "measurement_method": "MEASURED",
                "artifact_sha256": "d".repeat(64)
            }
        ]);

        manifest["safety"]["standard_edition"] =
            json!("synthetic test basis; not a real certification");
        manifest["safety"]["exposure_assessment_status"] = json!("ASSESSED");
        manifest["safety"]["foreign_object_detection"] = json!("PASS");
        manifest["safety"]["unauthorized_receiver"] = json!("PASS");
        manifest["safety"]["communication_loss_response"] = json!("SAFE_DEENERGIZE");
        manifest["safety"]["thermal_maximum_c"] = json!(70.0);
        manifest["cyber_physical_security"] = json!({
            "communication_mode": "NETWORKED",
            "authentication_method": "PKI_MUTUAL",
            "mutual_authentication": true,
            "replay_protection": true,
            "freshness_mechanism": "COMBINED",
            "receiver_identity_bound": true,
            "firmware_integrity_verified": true,
            "sensor_plausibility_checks": true,
            "spoofed_receiver_rejected": "PASS",
            "replay_message_rejected": "PASS",
            "sensor_tampering_detected": "PASS",
            "communication_loss_fallback": "SAFE_DEENERGIZE",
            "p99_message_latency_ms": 4.0,
            "max_message_jitter_ms": 1.0,
            "max_message_age_ms": 20.0,
            "control_deadline_ms": 10.0,
            "max_accepted_command_age_ms": 25.0,
            "security_artifact_sha256": "e".repeat(64),
            "security_scenario_results": [
                {"scenario_id":"WPT-SEC-AUTH-SPOOF-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-CMD-REPLAY-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-CMD-STALE-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-FW-INTEGRITY-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-SENSOR-PHASE-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-SENSOR-CURRENT-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-TIMING-JITTER-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-COMM-DOS-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-CERT-REVOKE-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."},
                {"scenario_id":"WPT-SEC-RESONANT-SWITCH-001","outcome":"PASS","evidence_sha256":"e".repeat(64),"notes":"Synthetic unit-test fixture only."}
            ]
        });

        manifest["artifact_refs"] = json!([
            {
                "artifact_type": "RAW_WAVEFORM",
                "uri": "artifact://synthetic/raw",
                "sha256": "a".repeat(64),
                "media_type": "application/octet-stream",
                "description": "Synthetic known-truth waveform placeholder; not measured data."
            },
            {
                "artifact_type": "CALIBRATION_CERTIFICATE",
                "uri": "artifact://synthetic/calibration",
                "sha256": "b".repeat(64),
                "media_type": "application/pdf",
                "description": "Synthetic known-truth calibration placeholder; not a real certificate."
            },
            {
                "artifact_type": "SECURITY_REPORT",
                "uri": "artifact://synthetic/security",
                "sha256": "e".repeat(64),
                "media_type": "application/json",
                "description": "Synthetic security-test placeholder; not a real attack report."
            }
        ]);
        manifest["result"]["qualification"] = json!("QUALIFIED");
        manifest["result"]["notes"] =
            json!("Synthetic unit-test fixture only; not a physical experiment.");
        manifest
    }


    #[test]
    fn frozen_security_scenario_corpus_matches_rust_required_ids() {
        let source = include_str!(
            "../../../benchmarks/wpt-empirical/v1/cyber-physical-scenarios.jsonl"
        );
        let mut ids = Vec::new();
        for (index, line) in source.lines().enumerate() {
            let record: Value = serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("security scenario line {}: {error}", index + 1));
            let id = string_at(&record, &["scenario_id"])
                .expect("scenario_id required in frozen security corpus");
            ids.push(id.to_owned());
            assert_eq!(
                string_at(&record, &["schema_version"]),
                Some("wpt-security-scenario-v1")
            );
            assert!(nonempty(string_at(&record, &["preconditions"])));
            assert!(nonempty(string_at(&record, &["stimulus"])));
            assert!(nonempty(string_at(&record, &["expected_safe_response"])));
            assert!(nonempty(string_at(&record, &["failure_condition"])));
        }
        ids.sort();
        let mut expected = REQUIRED_SECURITY_SCENARIOS
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(ids, expected, "frozen scenario IDs and validator contract drifted");
    }


    #[test]
    fn run_schema_matches_validator_top_level_nested_fields_and_scenario_ids() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../benchmarks/wpt-empirical/v1/test-run.schema.json"
        ))
        .expect("run manifest schema JSON parses");
        let example: Value = serde_json::from_str(include_str!(
            "../../../benchmarks/wpt-empirical/v1/test-run.example.json"
        ))
        .expect("run manifest example JSON parses");

        let unique_strings = |value: &Value, path: &[&str]| -> BTreeSet<String> {
            value
                .pointer(&format!("/{}", path.join("/")))
                .and_then(Value::as_array)
                .expect("schema required array")
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        };
        let len_strings = |value: &Value, path: &[&str]| -> usize {
            value
                .pointer(&format!("/{}", path.join("/")))
                .and_then(Value::as_array)
                .expect("schema required array")
                .len()
        };

        let schema_top = unique_strings(&schema, &["required"]);
        let validator_top = TOP_LEVEL.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
        assert_eq!(schema_top, validator_top, "schema and validator top-level fields drifted");
        assert_eq!(
            len_strings(&schema, &["required"]),
            schema_top.len(),
            "top-level schema contains duplicate required fields"
        );

        let condition_keys = unique_strings(
            &schema,
            &["properties", "conditions", "required"],
        );
        assert_eq!(
            len_strings(&schema, &["properties", "conditions", "required"]),
            condition_keys.len(),
            "conditions schema contains duplicate required fields"
        );
        let validator_conditions = [
            "direction",
            "power_class",
            "commanded_power_w",
            "frequency_hz",
            "transfer_gap_m",
            "offset_x_m",
            "offset_y_m",
            "offset_z_m",
            "yaw_deg",
            "receiver_count",
            "active_receiver_count",
            "battery_voltage_v",
            "load_profile_id",
            "ambient_temperature_c",
            "relative_humidity_pct",
            "thermal_state",
            "operating_mode",
            "installation_type",
            "rated_apparent_power_va",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
        assert_eq!(
            condition_keys, validator_conditions,
            "conditions schema and validator contract drifted"
        );

        let resonant_keys = unique_strings(
            &schema,
            &["properties", "resonant_networks", "items", "required"],
        );
        assert_eq!(
            len_strings(&schema, &["properties", "resonant_networks", "items", "required"]),
            resonant_keys.len(),
            "resonant network schema contains duplicate required fields"
        );
        let validator_resonant = [
            "network_id",
            "nominal_frequency_hz",
            "commanded_state",
            "isolation_switch_state",
            "current_rms_a",
            "current_peak_a",
            "continuous_current_limit_a",
            "current_peak_limit_a",
            "voltage_rms_v",
            "voltage_peak_v",
            "voltage_limit_v",
            "voltage_peak_limit_v",
            "thermal_maximum_c",
            "thermal_limit_c",
            "core_material_id",
            "core_saturation_curve_ref",
            "core_temperature_c",
            "core_flux_density_peak_t",
            "core_flux_density_limit_t",
            "core_flux_assessment_method",
            "measurement_method",
            "artifact_sha256",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
        assert_eq!(
            resonant_keys, validator_resonant,
            "resonant network schema and validator contract drifted"
        );

        let security_keys = unique_strings(
            &schema,
            &["properties", "cyber_physical_security", "required"],
        );
        assert_eq!(
            len_strings(&schema, &["properties", "cyber_physical_security", "required"]),
            security_keys.len(),
            "cyber-physical security schema contains duplicate required fields"
        );
        let validator_security = [
            "communication_mode",
            "authentication_method",
            "mutual_authentication",
            "replay_protection",
            "receiver_identity_bound",
            "firmware_integrity_verified",
            "sensor_plausibility_checks",
            "spoofed_receiver_rejected",
            "replay_message_rejected",
            "sensor_tampering_detected",
            "communication_loss_fallback",
            "p99_message_latency_ms",
            "max_message_jitter_ms",
            "max_message_age_ms",
            "security_artifact_sha256",
            "freshness_mechanism",
            "control_deadline_ms",
            "max_accepted_command_age_ms",
            "security_scenario_results",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
        assert_eq!(
            security_keys, validator_security,
            "security schema and validator contract drifted"
        );

        let standards_keys = unique_strings(
            &schema,
            &["properties", "standards_basis", "items", "required"],
        );
        assert_eq!(
            len_strings(&schema, &["properties", "standards_basis", "items", "required"]),
            standards_keys.len(),
            "standards basis schema contains duplicate required fields"
        );
        let validator_standards = [
            "document_id",
            "edition",
            "applicability",
            "scope_statement",
            "provisions_tested",
            "compliance_claim",
            "evidence_sha256",
            "issuing_body",
            "certificate_id",
            "notes",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
        assert_eq!(
            standards_keys, validator_standards,
            "standards-basis schema and validator contract drifted"
        );

        let schema_scenarios = unique_strings(
            &schema,
            &[
                "properties",
                "cyber_physical_security",
                "properties",
                "security_scenario_results",
                "items",
                "properties",
                "scenario_id",
                "enum",
            ],
        );
        let validator_scenarios = REQUIRED_SECURITY_SCENARIOS
            .iter()
            .map(|id| id.to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            schema_scenarios,
            validator_scenarios,
            "run schema and security scenario gate drifted"
        );
        assert_eq!(example["schema_version"].as_str(), Some("wpt-test-run-v1"));
        assert_eq!(example["standards_basis"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn not_run_template_is_structurally_valid_but_not_a_measurement() {
        let (disposition, errors) = validate_manifest(&template());
        assert_eq!(disposition, "NOT_RUN");
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn rejects_missing_nested_required_field_even_for_not_run_template() {
        let mut manifest = template();
        manifest["conditions"]
            .as_object_mut()
            .unwrap()
            .remove("frequency_hz");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("conditions.frequency_hz")));
    }

    #[test]
    fn rejects_unexpected_schema_field() {
        let mut manifest = template();
        manifest["system"]["unreviewed_claim"] = json!("not allowed");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("system.unreviewed_claim: unexpected field")));
    }

    #[test]
    fn synthetic_known_truth_qualified_record_passes() {
        let (disposition, errors) = validate_manifest(&qualified_fixture());
        assert_eq!(disposition, "QUALIFIED");
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn rejects_expired_calibration() {
        let mut manifest = qualified_fixture();
        manifest["metrology"]["meters"][0]["calibration_due_utc"] =
            json!("2026-09-30T00:00:00Z");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("calibration must cover")));
    }

    #[test]
    fn rejects_apparent_power_only_energy_accounting() {
        let mut manifest = qualified_fixture();
        manifest["metrology"]["power_basis"] = json!("APPARENT_POWER_ONLY");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("REAL_POWER required")));
    }

    #[test]
    fn rejects_energy_ledger_that_does_not_close_to_reported_residual() {
        let mut manifest = qualified_fixture();
        manifest["energy_balance"]["residual_energy_j"] = json!(500_000.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("ledger mismatch")));
    }

    #[test]
    fn rejects_residual_above_steady_state_target() {
        let mut manifest = qualified_fixture();
        manifest["energy_balance"]["useful_energy_j"] = json!(39_000_000.0);
        manifest["energy_balance"]["residual_energy_j"] = json!(700_000.0);
        manifest["energy_balance"]["residual_fraction_pct"] =
            json!(700_000.0 / 43_200_000.0 * 100.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("exceeds proposed 0.500%")));
    }

    #[test]
    fn rejects_failed_safety_gate() {
        let mut manifest = qualified_fixture();
        manifest["safety"]["unauthorized_receiver"] = json!("FAIL");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("unauthorized_receiver")));
    }

    #[test]
    fn accepts_measured_inactive_branch_below_one_percent_rating() {
        let (disposition, errors) = validate_manifest(&qualified_fixture());
        assert_eq!(disposition, "QUALIFIED");
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn rejects_modelled_inactive_branch_current_as_qualification_evidence() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][1]["measurement_method"] = json!("SIMULATED");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("requires MEASURED evidence")));
    }

    #[test]
    fn rejects_cross_frequency_inactive_loop_current_above_target() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][1]["current_rms_a"] = json!(17.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("exceeds proposed 1.000% target")));
    }

    #[test]
    fn rejects_simultaneous_active_networks_when_not_allowed() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][1]["commanded_state"] = json!("ACTIVE");
        manifest["resonant_networks"][1]["isolation_switch_state"] = json!("CLOSED");
        manifest["resonant_networks"][1]["current_rms_a"] = json!(12.0);
        manifest["resonant_networks"][1]["voltage_rms_v"] = json!(400.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("exactly one active branch")));
    }

    #[test]
    fn rejects_resonant_branch_peak_current_over_rating() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][0]["current_peak_a"] = json!(50.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("peak current missing or exceeds peak rating")));
    }


    #[test]
    fn rejects_core_flux_above_proposed_hot_state_margin() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][0]["core_flux_density_peak_t"] = json!(0.40);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("exceeds proposed hot-state 80.000% limit")));
    }

    #[test]
    fn rejects_simulation_only_core_flux_for_physical_qualification() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][0]["core_flux_assessment_method"] = json!("SIMULATED");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("calculation from measured waveform")));
    }

    #[test]
    fn rejects_resonant_branch_temperature_over_limit() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][0]["thermal_maximum_c"] = json!(105.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("exceeds branch thermal limit")));
    }

    #[test]
    fn rejects_faulted_resonant_isolation_switch() {
        let mut manifest = qualified_fixture();
        manifest["resonant_networks"][1]["isolation_switch_state"] = json!("FAULT");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("faulted switch state")));
    }


    #[test]
    fn rejects_replayed_power_command_in_networked_mode() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["replay_message_rejected"] = json!("FAIL");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("replay control test must pass")));
    }

    #[test]
    fn rejects_spoofed_receiver_acceptance() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["spoofed_receiver_rejected"] = json!("FAIL");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("spoofed receiver must be rejected")));
    }

    #[test]
    fn rejects_network_latency_over_declared_control_deadline() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["p99_message_latency_ms"] = json!(12.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("must meet positive control deadline")));
    }

    #[test]
    fn rejects_unsafe_communication_loss_fallback() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["communication_loss_fallback"] = json!("UNSAFE");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("safe fallback required")));
    }

    #[test]
    fn rejects_firmware_integrity_failure() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["firmware_integrity_verified"] = json!(false);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("verified firmware integrity required")));
    }


    #[test]
    fn rejects_missing_security_report_artifact_binding() {
        let mut manifest = qualified_fixture();
        manifest["artifact_refs"][2]["sha256"] = json!("f".repeat(64));
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("must bind a SECURITY_REPORT artifact")));
    }

    #[test]
    fn rejects_missing_network_replay_freshness_mechanism() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["freshness_mechanism"] = json!("UNKNOWN");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("replay-safe freshness mechanism required")));
    }



    #[test]
    fn rejects_security_scenario_digest_not_bound_to_report() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["security_scenario_results"][0]["evidence_sha256"] =
            json!("f".repeat(64));
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("PASS evidence must bind the run SECURITY_REPORT digest")));
    }

    #[test]
    fn rejects_missing_required_security_scenario() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["security_scenario_results"]
            .as_array_mut().unwrap().pop();
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("missing required scenario")));
    }

    #[test]
    fn rejects_security_scenario_not_run() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["security_scenario_results"][0]["outcome"] =
            json!("NOT_RUN");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("qualified run must PASS")));
    }

    #[test]
    fn rejects_security_scenario_without_evidence_digest() {
        let mut manifest = qualified_fixture();
        manifest["cyber_physical_security"]["security_scenario_results"][0]["evidence_sha256"] =
            Value::Null;
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("passing scenario requires evidence digest")));
    }

    #[test]
    fn rejects_duplicate_security_scenario_id() {
        let mut manifest = qualified_fixture();
        let id = manifest["cyber_physical_security"]["security_scenario_results"][0]["scenario_id"].clone();
        manifest["cyber_physical_security"]["security_scenario_results"][1]["scenario_id"] = id;
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("unknown or duplicate scenario")));
    }


    #[test]
    fn rejects_j2954_normative_claim_for_bidirectional_operation() {
        let mut manifest = qualified_fixture();
        manifest["conditions"]["direction"] = json!("VEHICLE_TO_GRID");
        manifest["standards_basis"][0]["applicability"] = json!("NORMATIVE_REQUIREMENT");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("requires GRID_TO_VEHICLE")));
    }

    #[test]
    fn rejects_j2954_normative_claim_above_11_kva() {
        let mut manifest = qualified_fixture();
        manifest["conditions"]["rated_apparent_power_va"] = json!(30_000.0);
        manifest["standards_basis"][0]["applicability"] = json!("NORMATIVE_REQUIREMENT");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("published <=11 kVA scope")));
    }


    #[test]
    fn rejects_j2954_normative_claim_for_flush_mounted_installation() {
        let mut manifest = qualified_fixture();
        manifest["conditions"]["installation_type"] = json!("FLUSH_MOUNTED");
        manifest["standards_basis"][0]["applicability"] = json!("NORMATIVE_REQUIREMENT");
        manifest["conditions"]["rated_apparent_power_va"] = json!(10_000.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("requires SURFACE_MOUNTED installation")));
    }

    #[test]
    fn rejects_j2954_normative_claim_for_dynamic_operation() {
        let mut manifest = qualified_fixture();
        manifest["conditions"]["operating_mode"] = json!("DYNAMIC");
        manifest["standards_basis"][0]["applicability"] = json!("NORMATIVE_REQUIREMENT");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("requires STATIC operation")));
    }


    #[test]
    fn rejects_j2954_3_as_normative_before_issued() {
        let mut manifest = qualified_fixture();
        manifest["standards_basis"][0]["document_id"] = json!("SAE J2954/3");
        manifest["standards_basis"][0]["edition"] = json!("planned");
        manifest["standards_basis"][0]["applicability"] = json!("NORMATIVE_REQUIREMENT");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("tracked as planned/unissued")));
    }

    #[test]
    fn rejects_j2954_2_as_complete_normative_specification() {
        let mut manifest = qualified_fixture();
        manifest["standards_basis"][0]["document_id"] = json!("SAE J2954/2");
        manifest["standards_basis"][0]["edition"] = json!("J2954/2_202212");
        manifest["standards_basis"][0]["applicability"] = json!("NORMATIVE_REQUIREMENT");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("Information Report")));
    }

    #[test]
    fn rejects_independent_certification_without_certificate_artifact() {
        let mut manifest = qualified_fixture();
        manifest["standards_basis"][0]["compliance_claim"] = json!("INDEPENDENTLY_CERTIFIED");
        manifest["standards_basis"][0]["evidence_sha256"] = json!("f".repeat(64));
        manifest["standards_basis"][0]["issuing_body"] = json!("synthetic issuer");
        manifest["standards_basis"][0]["certificate_id"] = json!("synthetic certificate");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("CERTIFICATION_EVIDENCE artifact")));
    }


    fn dynamic_fixture() -> Value {
        let mut manifest = qualified_fixture();
        manifest["conditions"]["operating_mode"] = json!("DYNAMIC");
        manifest["motion_profile"] = json!({
            "profile_id":"SYNTHETIC-DWPT-90KMH-001",
            "route_distance_m":10_000.0,
            "elapsed_time_s":400.0,
            "speed_mean_mps":25.0,
            "speed_min_mps":20.0,
            "speed_max_mps":30.0,
            "longitudinal_acceleration_abs_max_mps2":1.2,
            "transmitter_pitch_m":3.0,
            "segment_count_total":100,
            "segment_count_energized":98,
            "segment_activation_attempts":100,
            "segment_missed_activations":2,
            "false_activation_count":0,
            "roadway_energy_source_j":20_000_000.0,
            "vehicle_dc_energy_received_j":17_000_000.0,
            "battery_energy_gain_j":16_000_000.0,
            "vehicle_propulsion_energy_used_j":15_000_000.0,
            "roadway_idle_energy_j":500_000.0,
            "roadway_energy_per_km_wh_km":472.2222222222,
            "battery_net_energy_per_km_wh_km":27.7777777778,
            "energy_sufficiency_ratio":1.0666666667,
            "segment_activation_success_fraction":0.98,
            "speed_profile_sha256":"f".repeat(64),
            "segment_event_log_sha256":"e".repeat(64)
        });
        manifest
    }

    #[test]
    fn accepts_dynamic_route_energy_ledger_known_truth() {
        let (disposition, errors) = validate_manifest(&dynamic_fixture());
        assert_eq!(disposition, "QUALIFIED");
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn rejects_dynamic_energy_per_kilometer_mismatch() {
        let mut manifest = dynamic_fixture();
        manifest["motion_profile"]["roadway_energy_per_km_wh_km"] = json!(600.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("does not match route energy/event ledger")));
    }

    #[test]
    fn rejects_false_segment_activation() {
        let mut manifest = dynamic_fixture();
        manifest["motion_profile"]["false_activation_count"] = json!(1);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("zero false/unauthorized segment activations")));
    }

    #[test]
    fn rejects_malformed_dynamic_speed_profile() {
        let mut manifest = dynamic_fixture();
        manifest["motion_profile"]["speed_min_mps"] = json!(35.0);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("speed_min <= speed_mean <= speed_max")));
    }

    #[test]
    fn rejects_receiver_count_inversion() {
        let mut manifest = qualified_fixture();
        manifest["conditions"]["receiver_count"] = json!(1);
        manifest["conditions"]["active_receiver_count"] = json!(2);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("active <= total")));
    }

    #[test]
    fn rejects_duplicate_loss_stage_double_counting() {
        let mut manifest = qualified_fixture();
        manifest["energy_balance"]["loss_terms"] = json!([
            {"stage_id":"inverter","energy_j":1.0,"evidence_kind":"MEASURED"},
            {"stage_id":"inverter","energy_j":1.0,"evidence_kind":"MEASURED"}
        ]);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("stage_id: missing or duplicate")));
    }

    #[test]
    fn rejects_missing_raw_waveform_artifact() {
        let mut manifest = qualified_fixture();
        manifest["artifact_refs"] = json!([
            {
                "artifact_type": "CALIBRATION_CERTIFICATE",
                "uri": "artifact://synthetic/calibration",
                "sha256": "b".repeat(64),
                "media_type": "application/pdf",
                "description": "Synthetic placeholder."
            }
        ]);
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("RAW_WAVEFORM artifact required")));
    }

    #[test]
    fn rejects_invalid_utc_dates() {
        assert!(parse_utc_seconds("2026-02-29T00:00:00Z").is_none());
        assert!(parse_utc_seconds("2024-02-29T00:00:00Z").is_some());
        assert!(parse_utc_seconds("2026-01-01T00:00:00+02:00").is_none());
    }

    #[test]
    fn rejects_duplicate_meter_ids() {
        let mut manifest = qualified_fixture();
        let duplicate_id = manifest["metrology"]["meters"][0]["meter_id"].clone();
        manifest["metrology"]["meters"][1]["meter_id"] = duplicate_id;
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("meter_id: missing or duplicate")));
    }

    #[test]
    fn rejects_invalid_artifact_digest() {
        let mut manifest = qualified_fixture();
        manifest["artifact_refs"][0]["sha256"] = json!("not-a-hash");
        let (_, errors) = validate_manifest(&manifest);
        assert!(errors.iter().any(|e| e.contains("valid SHA-256 required")));
    }
}
