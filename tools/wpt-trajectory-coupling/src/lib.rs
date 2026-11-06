// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synchronized synthetic trajectory contract for dynamic WPT.
//!
//! This module derives relative geometry, wheel-path proximity and pass-level
//! energy integrals from one ordered trajectory. It does not infer coupling,
//! pavement stress, fatigue or battery-terminal energy.

use serde_json::Value;
use std::collections::BTreeSet;

const PPM: u64 = 1_000_000;
const MAX_SAMPLES: usize = 1024;
const MAX_WHEELS: usize = 16;
const MAX_SAMPLE_GAP_MS: u64 = 60_000;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn uint_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn int_field(value: &Value, key: &str, path: &str) -> Result<i64, String> {
    field(value, key, path)?
        .as_i64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a signed integer"))
}

fn optional_number(value: &Value, key: &str, path: &str) -> Result<Option<f64>, String> {
    match field(value, key, path)? {
        Value::Null => Ok(None),
        Value::Number(number) => number
            .as_f64()
            .filter(|value| value.is_finite())
            .map(Some)
            .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a finite number or null")),
        _ => Err(fail(
            &format!("{path}.{key}"),
            "must be a finite number or null",
        )),
    }
}

#[derive(Debug, Clone)]
struct Wheel {
    wheel_id: String,
    lateral_offset_mm: i64,
}

#[derive(Debug, Clone)]
struct Sample {
    sample_index: usize,
    time_ms: u64,
    vehicle_s_mm: i64,
    vehicle_center_y_mm: i64,
    speed_mm_per_s: u64,
    roadway_dc_input_power_w: u64,
    vehicle_receiver_dc_output_power_w: u64,
    coil_temperature_c: Option<f64>,
    pavement_interface_temperature_c: Option<f64>,
    ambient_temperature_c: Option<f64>,
    wheel_load_total_kn: f64,
}

#[derive(Debug, Clone)]
struct Scenario {
    episode_id: String,
    frame_id: String,
    vehicle_id: String,
    receiver_offset_longitudinal_mm: i64,
    receiver_offset_lateral_mm: i64,
    asset_id: String,
    segment_id: String,
    coil_id: String,
    coil_center_s_mm: i64,
    coil_center_y_mm: i64,
    active_zone_start_s_mm: i64,
    active_zone_end_s_mm: i64,
    alignment_limit_mm: u64,
    maximum_kinematic_mismatch_ppm: u64,
    wheels: Vec<Wheel>,
    samples: Vec<Sample>,
}

fn checked_add(left: i64, right: i64, path: &str) -> Result<i64, String> {
    left.checked_add(right)
        .ok_or_else(|| fail(path, "coordinate arithmetic overflowed"))
}

fn parse_scenario(value: &Value) -> Result<Scenario, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "episode_id",
            "evidence_kind",
            "coordinate_frame",
            "vehicle",
            "coil",
            "wheels",
            "declared_lateral_alignment_limit_mm",
            "max_sample_gap_ms",
            "max_kinematic_mismatch_ppm",
            "samples",
            "quality_flags",
        ],
        "$",
    )?;
    if string_field(value, "schema_version", "$")? != "wpt-trajectory-coupling-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    if string_field(value, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth trajectories only",
        ));
    }
    let flags = field(value, "quality_flags", "$")?
        .as_array()
        .ok_or_else(|| fail("$.quality_flags", "must be an array"))?;
    if flags.len() != 1 || flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 requires exactly SYNTHETIC_ONLY",
        ));
    }

    let frame = field(value, "coordinate_frame", "$")?;
    exact_keys(
        frame,
        &["frame_id", "longitudinal_positive", "lateral_positive"],
        "$.coordinate_frame",
    )?;
    let frame_id = string_field(frame, "frame_id", "$.coordinate_frame")?.to_string();
    if string_field(frame, "longitudinal_positive", "$.coordinate_frame")? != "FORWARD"
        || string_field(frame, "lateral_positive", "$.coordinate_frame")? != "LEFT"
    {
        return Err(fail(
            "$.coordinate_frame",
            "v0 requires longitudinal FORWARD and lateral LEFT axes",
        ));
    }

    let vehicle = field(value, "vehicle", "$")?;
    exact_keys(
        vehicle,
        &[
            "vehicle_id",
            "receiver_offset_longitudinal_mm",
            "receiver_offset_lateral_mm",
        ],
        "$.vehicle",
    )?;
    let vehicle_id = string_field(vehicle, "vehicle_id", "$.vehicle")?.to_string();
    let receiver_offset_longitudinal_mm =
        int_field(vehicle, "receiver_offset_longitudinal_mm", "$.vehicle")?;
    let receiver_offset_lateral_mm = int_field(vehicle, "receiver_offset_lateral_mm", "$.vehicle")?;

    let coil = field(value, "coil", "$")?;
    exact_keys(
        coil,
        &[
            "asset_id",
            "segment_id",
            "coil_id",
            "center_s_mm",
            "center_y_mm",
            "active_zone_start_s_mm",
            "active_zone_end_s_mm",
        ],
        "$.coil",
    )?;
    let asset_id = string_field(coil, "asset_id", "$.coil")?.to_string();
    let segment_id = string_field(coil, "segment_id", "$.coil")?.to_string();
    let coil_id = string_field(coil, "coil_id", "$.coil")?.to_string();
    let coil_center_s_mm = int_field(coil, "center_s_mm", "$.coil")?;
    let coil_center_y_mm = int_field(coil, "center_y_mm", "$.coil")?;
    let active_zone_start_s_mm = int_field(coil, "active_zone_start_s_mm", "$.coil")?;
    let active_zone_end_s_mm = int_field(coil, "active_zone_end_s_mm", "$.coil")?;
    if active_zone_start_s_mm >= active_zone_end_s_mm {
        return Err(fail(
            "$.coil",
            "active zone start must be less than active zone end",
        ));
    }

    let alignment_limit = uint_field(value, "declared_lateral_alignment_limit_mm", "$")?;
    let max_sample_gap_ms = uint_field(value, "max_sample_gap_ms", "$")?;
    if max_sample_gap_ms == 0 || max_sample_gap_ms > MAX_SAMPLE_GAP_MS {
        return Err(fail(
            "$.max_sample_gap_ms",
            format!("must be in 1..={MAX_SAMPLE_GAP_MS}"),
        ));
    }
    let max_kinematic_mismatch_ppm = uint_field(value, "max_kinematic_mismatch_ppm", "$")?;
    if max_kinematic_mismatch_ppm > PPM {
        return Err(fail(
            "$.max_kinematic_mismatch_ppm",
            "must be between zero and one million ppm",
        ));
    }

    let wheel_rows = field(value, "wheels", "$")?
        .as_array()
        .ok_or_else(|| fail("$.wheels", "must be an array"))?;
    if wheel_rows.len() < 2 || wheel_rows.len() > MAX_WHEELS {
        return Err(fail(
            "$.wheels",
            format!("must contain 2..={MAX_WHEELS} wheels"),
        ));
    }
    let mut wheels = Vec::with_capacity(wheel_rows.len());
    let mut wheel_ids = BTreeSet::new();
    for (index, row) in wheel_rows.iter().enumerate() {
        let path = format!("$.wheels[{index}]");
        exact_keys(
            row,
            &["wheel_id", "lateral_offset_from_vehicle_center_mm"],
            &path,
        )?;
        let wheel_id = string_field(row, "wheel_id", &path)?.to_string();
        if !wheel_ids.insert(wheel_id.clone()) {
            return Err(fail(&path, format!("duplicate wheel_id {wheel_id}")));
        }
        let lateral_offset_mm = int_field(row, "lateral_offset_from_vehicle_center_mm", &path)?;
        wheels.push(Wheel {
            wheel_id,
            lateral_offset_mm,
        });
    }
    wheels.sort_by_key(|wheel| wheel.wheel_id.clone());

    let sample_rows = field(value, "samples", "$")?
        .as_array()
        .ok_or_else(|| fail("$.samples", "must be an array"))?;
    if sample_rows.len() < 2 || sample_rows.len() > MAX_SAMPLES {
        return Err(fail(
            "$.samples",
            format!("must contain 2..={MAX_SAMPLES} samples"),
        ));
    }
    let expected_wheels = wheels
        .iter()
        .map(|wheel| wheel.wheel_id.clone())
        .collect::<BTreeSet<_>>();
    let mut samples = Vec::with_capacity(sample_rows.len());
    let mut previous_time_ms = None::<u64>;
    let mut previous_vehicle_s_mm = None::<i64>;

    for (index, row) in sample_rows.iter().enumerate() {
        let path = format!("$.samples[{index}]");
        exact_keys(
            row,
            &[
                "sample_index",
                "time_ms",
                "vehicle_s_mm",
                "vehicle_center_y_mm",
                "speed_mm_per_s",
                "roadway_dc_input_power_w",
                "vehicle_receiver_dc_output_power_w",
                "coil_temperature_c",
                "pavement_interface_temperature_c",
                "ambient_temperature_c",
                "wheel_loads",
            ],
            &path,
        )?;
        let sample_index = usize::try_from(uint_field(row, "sample_index", &path)?)
            .map_err(|_| fail(&path, "sample_index exceeds the platform index range"))?;
        if sample_index != index {
            return Err(fail(
                &path,
                format!("sample_index must equal its ordered array index {index}"),
            ));
        }
        let time_ms = uint_field(row, "time_ms", &path)?;
        let vehicle_s_mm = int_field(row, "vehicle_s_mm", &path)?;
        let vehicle_center_y_mm = int_field(row, "vehicle_center_y_mm", &path)?;
        let speed_mm_per_s = uint_field(row, "speed_mm_per_s", &path)?;
        if speed_mm_per_s == 0 {
            return Err(fail(&path, "speed_mm_per_s must be positive"));
        }
        if let Some(previous) = previous_time_ms
            && time_ms <= previous
        {
            return Err(fail(&path, "time_ms must be strictly increasing"));
        }
        if let Some(previous) = previous_time_ms
            && time_ms - previous > max_sample_gap_ms
        {
            return Err(fail(
                &path,
                format!(
                    "sample gap {} ms exceeds declared maximum {max_sample_gap_ms} ms",
                    time_ms - previous
                ),
            ));
        }
        if let Some(previous) = previous_vehicle_s_mm
            && vehicle_s_mm <= previous
        {
            return Err(fail(
                &path,
                "v0 forward-pass vehicle_s_mm must be strictly increasing",
            ));
        }
        previous_time_ms = Some(time_ms);
        previous_vehicle_s_mm = Some(vehicle_s_mm);

        let roadway_dc_input_power_w = uint_field(row, "roadway_dc_input_power_w", &path)?;
        let vehicle_receiver_dc_output_power_w =
            uint_field(row, "vehicle_receiver_dc_output_power_w", &path)?;
        let coil_temperature_c = optional_number(row, "coil_temperature_c", &path)?;
        let pavement_interface_temperature_c =
            optional_number(row, "pavement_interface_temperature_c", &path)?;
        let ambient_temperature_c = optional_number(row, "ambient_temperature_c", &path)?;

        let load_rows = field(row, "wheel_loads", &path)?
            .as_array()
            .ok_or_else(|| fail(&format!("{path}.wheel_loads"), "must be an array"))?;
        let mut seen_wheels = BTreeSet::new();
        let mut wheel_load_total_kn = 0.0_f64;
        for (wheel_index, load) in load_rows.iter().enumerate() {
            let load_path = format!("{path}.wheel_loads[{wheel_index}]");
            exact_keys(load, &["wheel_id", "vertical_load_kn"], &load_path)?;
            let wheel_id = string_field(load, "wheel_id", &load_path)?.to_string();
            if !expected_wheels.contains(&wheel_id) || !seen_wheels.insert(wheel_id.clone()) {
                return Err(fail(
                    &load_path,
                    format!("unknown or duplicate wheel_id {wheel_id}"),
                ));
            }
            let vertical_load_kn = field(load, "vertical_load_kn", &load_path)?
                .as_f64()
                .filter(|load| load.is_finite() && *load > 0.0)
                .ok_or_else(|| fail(&load_path, "vertical_load_kn must be finite and positive"))?;
            wheel_load_total_kn += vertical_load_kn;
        }
        if seen_wheels != expected_wheels {
            return Err(fail(
                &format!("{path}.wheel_loads"),
                "every declared wheel must have exactly one load observation",
            ));
        }
        if !wheel_load_total_kn.is_finite() {
            return Err(fail(
                &format!("{path}.wheel_loads"),
                "total wheel load is not finite",
            ));
        }

        samples.push(Sample {
            sample_index,
            time_ms,
            vehicle_s_mm,
            vehicle_center_y_mm,
            speed_mm_per_s,
            roadway_dc_input_power_w,
            vehicle_receiver_dc_output_power_w,
            coil_temperature_c,
            pavement_interface_temperature_c,
            ambient_temperature_c,
            wheel_load_total_kn,
        });
    }

    let mut maximum_kinematic_mismatch_ppm = 0_u64;
    for pair in samples.windows(2) {
        let delta_s_mm = pair[1]
            .vehicle_s_mm
            .checked_sub(pair[0].vehicle_s_mm)
            .ok_or_else(|| fail("$.samples", "forward progress difference overflowed"))?;
        let delta_t_ms = pair[1].time_ms - pair[0].time_ms;
        let observed_speed_mm_per_s = (delta_s_mm as u128 * 1000_u128) / delta_t_ms as u128;
        let expected_speed_mm_per_s =
            (pair[0].speed_mm_per_s as u128 + pair[1].speed_mm_per_s as u128) / 2;
        let mismatch_ppm = (observed_speed_mm_per_s.abs_diff(expected_speed_mm_per_s)
            * PPM as u128)
            / expected_speed_mm_per_s;
        let mismatch_ppm = mismatch_ppm.min(u64::MAX as u128) as u64;
        maximum_kinematic_mismatch_ppm = maximum_kinematic_mismatch_ppm.max(mismatch_ppm);
        if mismatch_ppm > max_kinematic_mismatch_ppm {
            return Err(fail(
                "$.samples",
                format!(
                    "odometry/speed mismatch {mismatch_ppm} ppm exceeds declared maximum {max_kinematic_mismatch_ppm} ppm"
                ),
            ));
        }
    }

    Ok(Scenario {
        episode_id: string_field(value, "episode_id", "$")?.to_string(),
        frame_id,
        vehicle_id,
        receiver_offset_longitudinal_mm,
        receiver_offset_lateral_mm,
        asset_id,
        segment_id,
        coil_id,
        coil_center_s_mm,
        coil_center_y_mm,
        active_zone_start_s_mm,
        active_zone_end_s_mm,
        alignment_limit_mm: alignment_limit,
        maximum_kinematic_mismatch_ppm,
        wheels,
        samples,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelPath {
    pub wheel_id: String,
    pub lateral_offset_from_coil_center_mm: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleWitness {
    pub sample_index: usize,
    pub speed_mm_per_s: u64,
    pub receiver_longitudinal_offset_from_coil_mm: i64,
    pub receiver_lateral_offset_from_coil_mm: i64,
    pub receiver_inside_declared_active_zone: bool,
    pub receiver_inside_declared_lateral_limit: bool,
    pub wheel_paths: Vec<WheelPath>,
    pub total_vertical_wheel_load_millikn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryCouplingReport {
    pub episode_id: String,
    pub frame_id: String,
    pub vehicle_id: String,
    pub asset_id: String,
    pub segment_id: String,
    pub coil_id: String,
    pub sample_count: usize,
    pub maximum_kinematic_mismatch_ppm: u64,
    pub active_zone_sample_count: usize,
    pub lateral_alignment_coverage_ppm: u64,
    pub maximum_absolute_receiver_lateral_offset_mm: u64,
    pub minimum_absolute_wheel_path_offset_mm: u64,
    pub roadway_dc_input_energy_uwh: u64,
    pub vehicle_receiver_dc_output_energy_uwh: u64,
    pub pass_level_conversion_efficiency_ppm: Option<u64>,
    pub total_vertical_wheel_load_impulse_micro_kn_s: u64,
    pub maximum_coil_temperature_millidegrees_c: Option<i64>,
    pub maximum_interface_temperature_millidegrees_c: Option<i64>,
    pub missing_coil_temperature_samples: usize,
    pub missing_interface_temperature_samples: usize,
    pub missing_ambient_temperature_samples: usize,
    pub witnesses: Vec<SampleWitness>,
    pub claim_ceiling: &'static str,
}

fn max_optional(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    values.flatten().reduce(f64::max)
}

fn max_optional_millidegrees(
    values: impl Iterator<Item = Option<f64>>,
    path: &str,
) -> Result<Option<i64>, String> {
    let Some(value) = max_optional(values) else {
        return Ok(None);
    };
    let millidegrees = (value * 1000.0).round();
    if !millidegrees.is_finite()
        || millidegrees < i64::MIN as f64
        || millidegrees >= i64::MAX as f64
    {
        return Err(fail(
            path,
            "temperature is outside the millidegree representation",
        ));
    }
    Ok(Some(millidegrees as i64))
}

fn integrate_power_uwh(samples: &[Sample], roadway_input: bool) -> Result<u64, String> {
    let mut energy_uwh = 0.0_f64;
    for pair in samples.windows(2) {
        let duration_ms = (pair[1].time_ms - pair[0].time_ms) as f64;
        let left = if roadway_input {
            pair[0].roadway_dc_input_power_w
        } else {
            pair[0].vehicle_receiver_dc_output_power_w
        };
        let right = if roadway_input {
            pair[1].roadway_dc_input_power_w
        } else {
            pair[1].vehicle_receiver_dc_output_power_w
        };
        energy_uwh += (left as f64 + right as f64) * duration_ms / 7.2;
    }
    if !energy_uwh.is_finite() || energy_uwh < 0.0 || energy_uwh >= u64::MAX as f64 {
        return Err("integrated pass energy exceeded the representable range".into());
    }
    Ok(energy_uwh.round() as u64)
}

fn integrate_wheel_load_impulse_micro_kn_s(samples: &[Sample]) -> Result<u64, String> {
    let mut impulse_micro_kn_s = 0.0_f64;
    for pair in samples.windows(2) {
        let duration_ms = (pair[1].time_ms - pair[0].time_ms) as f64;
        impulse_micro_kn_s +=
            (pair[0].wheel_load_total_kn + pair[1].wheel_load_total_kn) * duration_ms * 500.0;
    }
    if !impulse_micro_kn_s.is_finite()
        || impulse_micro_kn_s < 0.0
        || impulse_micro_kn_s >= u64::MAX as f64
    {
        return Err("integrated wheel-load impulse exceeded the representable range".into());
    }
    Ok(impulse_micro_kn_s.round() as u64)
}

/// Validate one synthetic pass and derive geometry and meter-boundary witnesses.
pub fn evaluate_trajectory(value: &Value) -> Result<TrajectoryCouplingReport, String> {
    let scenario = parse_scenario(value)?;
    let mut witnesses = Vec::with_capacity(scenario.samples.len());
    let mut active_zone_sample_count = 0_usize;
    let mut aligned_samples = 0_u64;
    let mut maximum_absolute_receiver_lateral_offset_mm = 0_u64;
    let mut minimum_absolute_wheel_path_offset_mm = u64::MAX;

    for sample in &scenario.samples {
        let receiver_s_mm = checked_add(
            sample.vehicle_s_mm,
            scenario.receiver_offset_longitudinal_mm,
            "$.samples.receiver_s_mm",
        )?;
        let receiver_y_mm = checked_add(
            sample.vehicle_center_y_mm,
            scenario.receiver_offset_lateral_mm,
            "$.samples.receiver_y_mm",
        )?;
        let receiver_longitudinal_offset_mm = receiver_s_mm
            .checked_sub(scenario.coil_center_s_mm)
            .ok_or_else(|| {
                fail(
                    "$.samples",
                    "receiver longitudinal offset arithmetic overflowed",
                )
            })?;
        let receiver_lateral_offset_mm = receiver_y_mm
            .checked_sub(scenario.coil_center_y_mm)
            .ok_or_else(|| fail("$.samples", "receiver lateral offset arithmetic overflowed"))?;
        let receiver_inside_declared_active_zone = receiver_s_mm >= scenario.active_zone_start_s_mm
            && receiver_s_mm <= scenario.active_zone_end_s_mm;
        if receiver_inside_declared_active_zone {
            active_zone_sample_count += 1;
        }
        let absolute_receiver_offset = receiver_lateral_offset_mm.unsigned_abs();
        maximum_absolute_receiver_lateral_offset_mm =
            maximum_absolute_receiver_lateral_offset_mm.max(absolute_receiver_offset);
        let receiver_inside_declared_lateral_limit =
            absolute_receiver_offset <= scenario.alignment_limit_mm;
        if receiver_inside_declared_lateral_limit {
            aligned_samples += 1;
        }

        let mut wheel_paths = Vec::with_capacity(scenario.wheels.len());
        for wheel in &scenario.wheels {
            let wheel_y_mm = checked_add(
                sample.vehicle_center_y_mm,
                wheel.lateral_offset_mm,
                "$.samples.wheel_y_mm",
            )?;
            let wheel_offset_mm = wheel_y_mm
                .checked_sub(scenario.coil_center_y_mm)
                .ok_or_else(|| fail("$.samples", "wheel path offset arithmetic overflowed"))?;
            minimum_absolute_wheel_path_offset_mm =
                minimum_absolute_wheel_path_offset_mm.min(wheel_offset_mm.unsigned_abs());
            wheel_paths.push(WheelPath {
                wheel_id: wheel.wheel_id.clone(),
                lateral_offset_from_coil_center_mm: wheel_offset_mm,
            });
        }
        wheel_paths.sort_by_key(|wheel| wheel.wheel_id.clone());

        let total_vertical_wheel_load_millikn = (sample.wheel_load_total_kn * 1000.0).round();
        if !total_vertical_wheel_load_millikn.is_finite()
            || total_vertical_wheel_load_millikn < 0.0
            || total_vertical_wheel_load_millikn >= u64::MAX as f64
        {
            return Err(fail(
                "$.samples.wheel_loads",
                "total vertical wheel load exceeded the representable range",
            ));
        }

        witnesses.push(SampleWitness {
            sample_index: sample.sample_index,
            speed_mm_per_s: sample.speed_mm_per_s,
            receiver_longitudinal_offset_from_coil_mm: receiver_longitudinal_offset_mm,
            receiver_lateral_offset_from_coil_mm: receiver_lateral_offset_mm,
            receiver_inside_declared_active_zone,
            receiver_inside_declared_lateral_limit,
            wheel_paths,
            total_vertical_wheel_load_millikn: total_vertical_wheel_load_millikn as u64,
        });
    }

    let roadway_dc_input_energy_uwh = integrate_power_uwh(&scenario.samples, true)?;
    let vehicle_receiver_dc_output_energy_uwh = integrate_power_uwh(&scenario.samples, false)?;
    let pass_level_conversion_efficiency_ppm = if roadway_dc_input_energy_uwh > 0
        && vehicle_receiver_dc_output_energy_uwh <= roadway_dc_input_energy_uwh
    {
        Some(
            ((vehicle_receiver_dc_output_energy_uwh as u128 * PPM as u128
                + roadway_dc_input_energy_uwh as u128 / 2)
                / roadway_dc_input_energy_uwh as u128) as u64,
        )
    } else {
        None
    };

    let sample_count = scenario.samples.len() as u64;
    let lateral_alignment_coverage_ppm = (aligned_samples * PPM + sample_count / 2) / sample_count;
    let maximum_coil_temperature_millidegrees_c = max_optional_millidegrees(
        scenario
            .samples
            .iter()
            .map(|sample| sample.coil_temperature_c),
        "$.samples.coil_temperature_c",
    )?;
    let maximum_interface_temperature_millidegrees_c = max_optional_millidegrees(
        scenario
            .samples
            .iter()
            .map(|sample| sample.pavement_interface_temperature_c),
        "$.samples.pavement_interface_temperature_c",
    )?;
    let missing_coil_temperature_samples = scenario
        .samples
        .iter()
        .filter(|sample| sample.coil_temperature_c.is_none())
        .count();
    let missing_interface_temperature_samples = scenario
        .samples
        .iter()
        .filter(|sample| sample.pavement_interface_temperature_c.is_none())
        .count();
    let missing_ambient_temperature_samples = scenario
        .samples
        .iter()
        .filter(|sample| sample.ambient_temperature_c.is_none())
        .count();

    Ok(TrajectoryCouplingReport {
        episode_id: scenario.episode_id,
        frame_id: scenario.frame_id,
        vehicle_id: scenario.vehicle_id,
        asset_id: scenario.asset_id,
        segment_id: scenario.segment_id,
        coil_id: scenario.coil_id,
        sample_count: sample_count as usize,
        maximum_kinematic_mismatch_ppm: scenario.maximum_kinematic_mismatch_ppm,
        active_zone_sample_count,
        lateral_alignment_coverage_ppm,
        maximum_absolute_receiver_lateral_offset_mm,
        minimum_absolute_wheel_path_offset_mm,
        roadway_dc_input_energy_uwh,
        vehicle_receiver_dc_output_energy_uwh,
        pass_level_conversion_efficiency_ppm,
        total_vertical_wheel_load_impulse_micro_kn_s: integrate_wheel_load_impulse_micro_kn_s(
            &scenario.samples,
        )?,
        maximum_coil_temperature_millidegrees_c,
        maximum_interface_temperature_millidegrees_c,
        missing_coil_temperature_samples,
        missing_interface_temperature_samples,
        missing_ambient_temperature_samples,
        witnesses,
        claim_ceiling: "Synthetic coordinate, load-exposure and trapezoidal DC-meter integration only; not a coupling, pavement stress/fatigue, thermal constitutive, battery-energy, safety or field-performance model.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-trajectory-coupling-v0/trajectory.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic WPT trajectory coupling scenario JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn known_truth_derives_geometry_energy_and_exposure() {
        let report = evaluate_trajectory(&fixture()).unwrap();
        assert_eq!(report.sample_count, 3);
        assert_eq!(report.maximum_kinematic_mismatch_ppm, 0);
        assert_eq!(report.active_zone_sample_count, 1);
        assert_eq!(report.witnesses[0].speed_mm_per_s, 10_000);
        assert_eq!(report.lateral_alignment_coverage_ppm, 666_667);
        assert_eq!(report.maximum_absolute_receiver_lateral_offset_mm, 220);
        assert_eq!(report.minimum_absolute_wheel_path_offset_mm, 600);
        assert_eq!(report.roadway_dc_input_energy_uwh, 27_777_778);
        assert_eq!(report.vehicle_receiver_dc_output_energy_uwh, 23_611_111);
        assert_eq!(report.pass_level_conversion_efficiency_ppm, Some(850_000));
        assert_eq!(
            report.total_vertical_wheel_load_impulse_micro_kn_s,
            84_000_000
        );
        assert_eq!(report.maximum_coil_temperature_millidegrees_c, Some(45_000));
        assert_eq!(
            report.maximum_interface_temperature_millidegrees_c,
            Some(35_000)
        );
        assert_eq!(report.missing_ambient_temperature_samples, 0);
        assert_eq!(
            report.witnesses[1].receiver_lateral_offset_from_coil_mm,
            120
        );
        assert!(report.witnesses[1].receiver_inside_declared_active_zone);
        assert_eq!(report.witnesses[1].wheel_paths.len(), 4);
    }

    #[test]
    fn common_coordinate_translation_preserves_relative_geometry() {
        let first = evaluate_trajectory(&fixture()).unwrap();
        let mut shifted = fixture();
        shifted["coil"]["center_s_mm"] = Value::from(25_000);
        shifted["coil"]["active_zone_start_s_mm"] = Value::from(24_000);
        shifted["coil"]["active_zone_end_s_mm"] = Value::from(26_000);
        for sample in shifted["samples"].as_array_mut().unwrap() {
            let s = sample["vehicle_s_mm"].as_i64().unwrap() + 20_000;
            sample["vehicle_s_mm"] = Value::from(s);
        }
        shifted["coil"]["center_y_mm"] = Value::from(-3000);
        for sample in shifted["samples"].as_array_mut().unwrap() {
            let y = sample["vehicle_center_y_mm"].as_i64().unwrap() - 3000;
            sample["vehicle_center_y_mm"] = Value::from(y);
        }
        let second = evaluate_trajectory(&shifted).unwrap();
        assert_eq!(
            first.lateral_alignment_coverage_ppm,
            second.lateral_alignment_coverage_ppm
        );
        assert_eq!(
            first.maximum_absolute_receiver_lateral_offset_mm,
            second.maximum_absolute_receiver_lateral_offset_mm
        );
        assert_eq!(
            first.minimum_absolute_wheel_path_offset_mm,
            second.minimum_absolute_wheel_path_offset_mm
        );
        assert_eq!(
            first.roadway_dc_input_energy_uwh,
            second.roadway_dc_input_energy_uwh
        );
        assert_eq!(first.witnesses, second.witnesses);
    }

    #[test]
    fn sample_order_and_forward_progress_are_not_silently_repaired() {
        let mut scenario = fixture();
        scenario["samples"].as_array_mut().unwrap().reverse();
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("sample_index")
        );

        let mut scenario = fixture();
        scenario["samples"][2]["vehicle_s_mm"] = Value::from(4000);
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("strictly increasing")
        );
    }

    #[test]
    fn odometry_and_speed_must_agree_within_declared_tolerance() {
        let mut scenario = fixture();
        for sample in scenario["samples"].as_array_mut().unwrap() {
            sample["speed_mm_per_s"] = Value::from(4000);
        }
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("odometry/speed mismatch")
        );
    }

    #[test]
    fn single_sample_clock_shift_breaks_kinematic_consistency() {
        let mut scenario = fixture();
        scenario["samples"][1]["time_ms"] = Value::from(1100);
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("odometry/speed mismatch")
        );
    }

    #[test]
    fn lateral_axis_sign_cannot_be_silently_inverted() {
        let mut scenario = fixture();
        scenario["coordinate_frame"]["lateral_positive"] = Value::String("RIGHT".into());
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("lateral LEFT axes")
        );
    }

    #[test]
    fn swapped_meter_channels_suppress_conversion_efficiency() {
        let mut scenario = fixture();
        for sample in scenario["samples"].as_array_mut().unwrap() {
            let input = sample["roadway_dc_input_power_w"].clone();
            sample["roadway_dc_input_power_w"] =
                sample["vehicle_receiver_dc_output_power_w"].clone();
            sample["vehicle_receiver_dc_output_power_w"] = input;
        }
        let report = evaluate_trajectory(&scenario).unwrap();
        assert!(report.vehicle_receiver_dc_output_energy_uwh > report.roadway_dc_input_energy_uwh);
        assert_eq!(report.pass_level_conversion_efficiency_ppm, None);
    }

    #[test]
    fn sample_gap_must_remain_within_declared_support() {
        let mut scenario = fixture();
        scenario["max_sample_gap_ms"] = Value::from(500);
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("exceeds declared maximum")
        );
    }

    #[test]
    fn wheel_channels_must_match_declared_geometry_exactly() {
        let mut missing = fixture();
        missing["samples"][1]["wheel_loads"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(
            evaluate_trajectory(&missing)
                .unwrap_err()
                .contains("every declared wheel")
        );

        let mut unknown = fixture();
        unknown["samples"][1]["wheel_loads"][0]["wheel_id"] = Value::String("unknown-wheel".into());
        assert!(
            evaluate_trajectory(&unknown)
                .unwrap_err()
                .contains("unknown or duplicate wheel_id")
        );
    }

    #[test]
    fn ambiguous_pass_energy_does_not_publish_efficiency() {
        let mut scenario = fixture();
        scenario["samples"][1]["vehicle_receiver_dc_output_power_w"] = Value::from(110_000);
        let report = evaluate_trajectory(&scenario).unwrap();
        assert_eq!(report.pass_level_conversion_efficiency_ppm, None);
        assert!(report.vehicle_receiver_dc_output_energy_uwh > report.roadway_dc_input_energy_uwh);
    }

    #[test]
    fn missing_thermal_channels_remain_explicit() {
        let mut scenario = fixture();
        scenario["samples"][1]["coil_temperature_c"] = Value::Null;
        scenario["samples"][2]["pavement_interface_temperature_c"] = Value::Null;
        scenario["samples"][0]["ambient_temperature_c"] = Value::Null;
        let report = evaluate_trajectory(&scenario).unwrap();
        assert_eq!(report.maximum_coil_temperature_millidegrees_c, Some(30_000));
        assert_eq!(
            report.maximum_interface_temperature_millidegrees_c,
            Some(35_000)
        );
        assert_eq!(report.missing_coil_temperature_samples, 1);
        assert_eq!(report.missing_interface_temperature_samples, 1);
        assert_eq!(report.missing_ambient_temperature_samples, 1);
    }

    #[test]
    fn duplicate_ids_unknown_fields_and_non_synthetic_evidence_fail_closed() {
        let mut scenario = fixture();
        scenario["wheels"][1]["wheel_id"] = scenario["wheels"][0]["wheel_id"].clone();
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("duplicate wheel_id")
        );

        let mut scenario = fixture();
        scenario["unmodelled_alignment_override"] = Value::Bool(true);
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("unknown property")
        );

        let mut scenario = fixture();
        scenario["evidence_kind"] = Value::String("MEASURED".into());
        assert!(
            evaluate_trajectory(&scenario)
                .unwrap_err()
                .contains("synthetic known-truth")
        );
    }
}
