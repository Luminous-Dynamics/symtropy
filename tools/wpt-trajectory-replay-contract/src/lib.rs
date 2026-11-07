// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed synchronization contract for dynamic WPT trajectory replay.
//!
//! One canonical vehicle pose sequence is projected into separate
//! electromagnetic and pavement evidence lanes. This crate validates lineage,
//! timing, frame identity, transform receipts and one-to-one coverage only.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_SAMPLES: usize = 512;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn unsigned_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn signed_field(value: &Value, key: &str, path: &str) -> Result<i64, String> {
    field(value, key, path)?
        .as_i64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an integer"))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a number"))?;
    if !number.is_finite() {
        return Err(fail(&format!("{path}.{key}"), "must be finite"));
    }
    Ok(number)
}

fn array_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    field(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))
}

fn validate_digest(digest: &str, path: &str) -> Result<(), String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(fail(path, "must be a lowercase SHA-256 digest"));
    }
    Ok(())
}

fn validate_sample(
    value: &Value,
    index: usize,
    sample_timestamps: &mut BTreeMap<String, u64>,
) -> Result<u64, String> {
    let path = format!("$.trajectory_samples[{index}]");
    exact_keys(
        value,
        &[
            "sample_id",
            "timestamp_ns",
            "x_mm",
            "y_mm",
            "z_mm",
            "yaw_deg",
            "pitch_deg",
            "roll_deg",
            "speed_mps",
            "axle_load_n",
            "left_wheel_y_mm",
            "right_wheel_y_mm",
        ],
        &path,
    )?;
    let sample_id = string_field(value, "sample_id", &path)?.to_string();
    let timestamp = unsigned_field(value, "timestamp_ns", &path)?;
    if sample_timestamps
        .insert(sample_id.clone(), timestamp)
        .is_some()
    {
        return Err(fail(&path, format!("duplicate sample_id {sample_id}")));
    }
    for key in [
        "x_mm",
        "y_mm",
        "z_mm",
        "yaw_deg",
        "pitch_deg",
        "roll_deg",
        "speed_mps",
        "axle_load_n",
        "left_wheel_y_mm",
        "right_wheel_y_mm",
    ] {
        number_field(value, key, &path)?;
    }
    if number_field(value, "speed_mps", &path)? < 0.0 {
        return Err(fail(&path, "speed_mps must be non-negative"));
    }
    if number_field(value, "axle_load_n", &path)? <= 0.0 {
        return Err(fail(&path, "axle_load_n must be positive"));
    }
    Ok(timestamp)
}

fn validate_projection_group(
    projections: &[Value],
    kind: &str,
    trajectory_id: &str,
    frame_id: &str,
    sample_timestamps: &BTreeMap<String, u64>,
) -> Result<BTreeSet<String>, String> {
    if projections.len() != sample_timestamps.len() {
        return Err(fail(
            &format!("$.{kind}"),
            format!("exactly one {kind} projection is required per trajectory sample"),
        ));
    }

    let electromagnetic = kind == "electromagnetic_projections";
    let allowed: &[&str] = if electromagnetic {
        &[
            "projection_id",
            "trajectory_id",
            "sample_id",
            "coordinate_frame_id",
            "transform_sha256",
            "gap_mm",
            "lateral_offset_mm",
            "coupling_ppm",
            "useful_power_w",
            "vehicle_dc_power_w",
            "measurement_timestamp_ns",
            "clock_offset_ns",
            "clock_uncertainty_ns",
        ]
    } else {
        &[
            "projection_id",
            "trajectory_id",
            "sample_id",
            "coordinate_frame_id",
            "transform_sha256",
            "temperature_c",
            "tensile_strain_microstrain",
            "heat_flux_w_m2",
            "interface_condition",
            "measurement_timestamp_ns",
            "clock_offset_ns",
            "clock_uncertainty_ns",
        ]
    };

    let mut projection_ids = BTreeSet::new();
    let mut projected_samples = BTreeSet::new();
    for (index, projection) in projections.iter().enumerate() {
        let path = format!("$.{kind}[{index}]");
        exact_keys(projection, allowed, &path)?;
        let projection_id = string_field(projection, "projection_id", &path)?.to_string();
        if !projection_ids.insert(projection_id.clone()) {
            return Err(fail(
                &path,
                format!("duplicate projection_id {projection_id}"),
            ));
        }
        if string_field(projection, "trajectory_id", &path)? != trajectory_id {
            return Err(fail(
                &path,
                "projection trajectory_id does not match canonical trajectory",
            ));
        }
        let sample_id = string_field(projection, "sample_id", &path)?.to_string();
        let canonical_timestamp = sample_timestamps
            .get(&sample_id)
            .ok_or_else(|| fail(&path, format!("unknown trajectory sample_id {sample_id}")))?;
        let measurement_timestamp = unsigned_field(projection, "measurement_timestamp_ns", &path)?;
        let clock_offset = signed_field(projection, "clock_offset_ns", &path)?;
        let clock_uncertainty = unsigned_field(projection, "clock_uncertainty_ns", &path)?;
        let corrected_timestamp = i128::from(measurement_timestamp) + i128::from(clock_offset);
        let timestamp_error = (corrected_timestamp - i128::from(*canonical_timestamp)).abs();
        if timestamp_error > i128::from(clock_uncertainty) {
            return Err(fail(
                &path,
                "corrected measurement timestamp exceeds declared clock uncertainty",
            ));
        }
        if !projected_samples.insert(sample_id.clone()) {
            return Err(fail(
                &path,
                format!("more than one projection references sample_id {sample_id}"),
            ));
        }
        if string_field(projection, "coordinate_frame_id", &path)? != frame_id {
            return Err(fail(
                &path,
                "projection coordinate frame does not match canonical trajectory frame",
            ));
        }
        validate_digest(
            string_field(projection, "transform_sha256", &path)?,
            &format!("{path}.transform_sha256"),
        )?;

        if electromagnetic {
            if number_field(projection, "gap_mm", &path)? < 0.0 {
                return Err(fail(&path, "gap_mm must be non-negative"));
            }
            let coupling = unsigned_field(projection, "coupling_ppm", &path)?;
            if coupling > 1_000_000 {
                return Err(fail(&path, "coupling_ppm must not exceed 1,000,000"));
            }
            if number_field(projection, "useful_power_w", &path)? < 0.0
                || number_field(projection, "vehicle_dc_power_w", &path)? < 0.0
            {
                return Err(fail(&path, "power measurements must be non-negative"));
            }
            number_field(projection, "lateral_offset_mm", &path)?;
        } else {
            number_field(projection, "temperature_c", &path)?;
            number_field(projection, "tensile_strain_microstrain", &path)?;
            number_field(projection, "heat_flux_w_m2", &path)?;
            let condition = string_field(projection, "interface_condition", &path)?;
            if !["INTACT", "DEBONDED", "CRACKED", "UNKNOWN"].contains(&condition) {
                return Err(fail(&path, "unsupported interface_condition"));
            }
        }
    }

    if projected_samples != sample_timestamps.keys().cloned().collect::<BTreeSet<_>>() {
        return Err(fail(
            &format!("$.{kind}"),
            "projection lane does not cover the canonical sample set exactly",
        ));
    }
    Ok(projected_samples)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryReplayReport {
    pub trajectory_id: String,
    pub sample_count: usize,
    pub electromagnetic_projection_count: usize,
    pub pavement_projection_count: usize,
    pub checked_sample_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

/// Validate shared trajectory lineage and one-to-one EM/pavement projection coverage.
pub fn validate_trajectory_episode(value: &Value) -> Result<TrajectoryReplayReport, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "trajectory_id",
            "evidence_kind",
            "coordinate_frame_id",
            "quality_flags",
            "trajectory_samples",
            "electromagnetic_projections",
            "pavement_projections",
        ],
        "$",
    )?;
    if string_field(value, "schema_version", "$")? != "wpt-trajectory-replay-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let trajectory_id = string_field(value, "trajectory_id", "$")?.to_string();
    if string_field(value, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth episodes only",
        ));
    }
    let frame_id = string_field(value, "coordinate_frame_id", "$")?.to_string();
    let flags = array_field(value, "quality_flags", "$")?;
    if flags.len() != 1 || flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 episodes must carry exactly SYNTHETIC_ONLY",
        ));
    }

    let sample_rows = array_field(value, "trajectory_samples", "$")?;
    if sample_rows.is_empty() || sample_rows.len() > MAX_SAMPLES {
        return Err(fail(
            "$.trajectory_samples",
            format!("must contain 1..={MAX_SAMPLES} samples"),
        ));
    }
    let mut sample_timestamps = BTreeMap::new();
    let mut previous_timestamp = None::<u64>;
    for (index, row) in sample_rows.iter().enumerate() {
        let timestamp = validate_sample(row, index, &mut sample_timestamps)?;
        if previous_timestamp.is_some_and(|previous| timestamp <= previous) {
            return Err(fail(
                &format!("$.trajectory_samples[{index}]"),
                "timestamp_ns must be strictly increasing",
            ));
        }
        previous_timestamp = Some(timestamp);
    }

    let electromagnetic = array_field(value, "electromagnetic_projections", "$")?;
    let pavement = array_field(value, "pavement_projections", "$")?;
    validate_projection_group(
        electromagnetic,
        "electromagnetic_projections",
        &trajectory_id,
        &frame_id,
        &sample_timestamps,
    )?;
    validate_projection_group(
        pavement,
        "pavement_projections",
        &trajectory_id,
        &frame_id,
        &sample_timestamps,
    )?;

    Ok(TrajectoryReplayReport {
        trajectory_id,
        sample_count: sample_rows.len(),
        electromagnetic_projection_count: electromagnetic.len(),
        pavement_projection_count: pavement.len(),
        checked_sample_ids: sample_timestamps.into_keys().collect(),
        claim_ceiling: "Synthetic trajectory lineage, timestamp order, coordinate-frame agreement and projection coverage only; does not validate electromagnetic coupling, pavement mechanics, thermal response, EMC, safety or field performance.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-trajectory-replay-v0/trajectory.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic WPT trajectory replay JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn known_truth_fixture_binds_both_lanes_to_the_same_pose_set() {
        let report = validate_trajectory_episode(&fixture()).unwrap();
        assert_eq!(report.sample_count, 3);
        assert_eq!(report.electromagnetic_projection_count, 3);
        assert_eq!(report.pavement_projection_count, 3);
        assert_eq!(
            report.checked_sample_ids,
            vec!["pose-000", "pose-001", "pose-002"]
        );
    }

    #[test]
    fn duplicate_or_non_monotonic_trajectory_samples_are_rejected() {
        let mut scenario = fixture();
        scenario["trajectory_samples"][1]["sample_id"] =
            scenario["trajectory_samples"][0]["sample_id"].clone();
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("duplicate sample_id")
        );

        let mut scenario = fixture();
        scenario["trajectory_samples"][2]["timestamp_ns"] = Value::from(50);
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("strictly increasing")
        );
    }

    #[test]
    fn projection_must_reference_a_canonical_sample() {
        let mut scenario = fixture();
        scenario["electromagnetic_projections"][0]["sample_id"] =
            Value::String("unrecorded-pose".into());
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("unknown trajectory sample_id")
        );
    }

    #[test]
    fn both_physics_lanes_must_use_the_canonical_coordinate_frame() {
        let mut scenario = fixture();
        scenario["electromagnetic_projections"][0]["coordinate_frame_id"] =
            Value::String("other-frame".into());
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("coordinate frame")
        );

        let mut scenario = fixture();
        scenario["pavement_projections"][0]["coordinate_frame_id"] =
            Value::String("other-frame".into());
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("coordinate frame")
        );
    }

    #[test]
    fn each_lane_requires_exactly_one_projection_per_pose() {
        let mut scenario = fixture();
        scenario["electromagnetic_projections"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("exactly one electromagnetic_projections")
        );

        let mut scenario = fixture();
        scenario["pavement_projections"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("exactly one pavement_projections")
        );
    }

    #[test]
    fn trajectory_identity_and_transform_digest_are_bound() {
        let mut scenario = fixture();
        scenario["pavement_projections"][0]["trajectory_id"] =
            Value::String("different-trajectory".into());
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("does not match canonical trajectory")
        );

        let mut scenario = fixture();
        scenario["electromagnetic_projections"][0]["transform_sha256"] =
            Value::String("not-a-digest".into());
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("lowercase SHA-256")
        );
    }

    #[test]
    fn projection_clock_offset_must_align_with_the_canonical_pose_within_uncertainty() {
        let mut scenario = fixture();
        scenario["electromagnetic_projections"][0]["measurement_timestamp_ns"] =
            Value::from(30_000_000);
        scenario["electromagnetic_projections"][0]["clock_offset_ns"] = Value::from(-30_000_000);
        scenario["electromagnetic_projections"][0]["clock_uncertainty_ns"] = Value::from(0);
        assert!(validate_trajectory_episode(&scenario).is_ok());

        let mut scenario = fixture();
        scenario["pavement_projections"][0]["measurement_timestamp_ns"] = Value::from(3);
        scenario["pavement_projections"][0]["clock_offset_ns"] = Value::from(0);
        scenario["pavement_projections"][0]["clock_uncertainty_ns"] = Value::from(1);
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("corrected measurement timestamp exceeds declared clock uncertainty")
        );
    }

    #[test]
    fn invalid_motion_and_non_synthetic_evidence_fail_closed() {
        let mut scenario = fixture();
        scenario["trajectory_samples"][0]["speed_mps"] = Value::from(-1);
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("speed_mps must be non-negative")
        );

        let mut scenario = fixture();
        scenario["evidence_kind"] = Value::String("MEASURED".into());
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("synthetic known-truth")
        );
    }

    #[test]
    fn unknown_properties_and_duplicate_projection_ids_fail_closed() {
        let mut scenario = fixture();
        scenario["trajectory_samples"][0]["invented_alignment_score"] = Value::from(1);
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("unknown property")
        );

        let mut scenario = fixture();
        let duplicate = scenario["electromagnetic_projections"][0]["projection_id"].clone();
        scenario["electromagnetic_projections"][1]["projection_id"] = duplicate;
        assert!(
            validate_trajectory_episode(&scenario)
                .unwrap_err()
                .contains("duplicate projection_id")
        );
    }
}
