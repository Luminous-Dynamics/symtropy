// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent verification of a synchronized WPT trajectory artifact bundle.
//!
//! This verifier checks exact raw bytes, SHA-256 receipts, canonical pose
//! identity/order, timestamp correction bounds, coordinate-frame lineage and
//! re-derived affine point transforms. It does not infer magnetic coupling,
//! pavement stress, heat transfer, fatigue or battery-terminal energy.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const PPM: i128 = 1_000_000;
const MAX_SAMPLES: usize = 4096;
const REQUIRED_ARTIFACTS: [&str; 5] = [
    "trajectory",
    "em_projection",
    "pavement_projection",
    "em_transform",
    "pavement_transform",
];

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    let map = object(value, path)?;
    for key in map.keys() {
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
        .filter(|value| !value.trim().is_empty())
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

fn array_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    field(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone)]
struct RawArtifact {
    artifact_id: String,
    sha256: String,
    bytes: Vec<u8>,
    json: Value,
}

fn read_artifact(
    artifacts: &Value,
    name: &str,
    maximum_bytes: u64,
    ids: &mut BTreeSet<String>,
) -> Result<RawArtifact, String> {
    let path = format!("$.artifacts.{name}");
    let value = field(artifacts, name, "$.artifacts")?;
    exact_keys(
        value,
        &["artifact_id", "media_type", "sha256", "bytes_base64"],
        &path,
    )?;
    let artifact_id = string_field(value, "artifact_id", &path)?.to_string();
    if !ids.insert(artifact_id.clone()) {
        return Err(fail(&path, format!("duplicate artifact_id {artifact_id}")));
    }
    if string_field(value, "media_type", &path)? != "application/json" {
        return Err(fail(
            &format!("{path}.media_type"),
            "v0 requires application/json",
        ));
    }
    let expected_digest = string_field(value, "sha256", &path)?.to_string();
    if !valid_sha256(&expected_digest) {
        return Err(fail(
            &format!("{path}.sha256"),
            "must be 64 lowercase hexadecimal characters",
        ));
    }
    let encoded = string_field(value, "bytes_base64", &path)?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|error| fail(&format!("{path}.bytes_base64"), error.to_string()))?;
    if bytes.is_empty() {
        return Err(fail(&path, "artifact bytes must not be empty"));
    }
    if bytes.len() as u128 > maximum_bytes as u128 {
        return Err(fail(
            &path,
            format!("artifact exceeds declared maximum of {maximum_bytes} bytes"),
        ));
    }
    let actual_digest = sha256_hex(&bytes);
    if actual_digest != expected_digest {
        return Err(fail(
            &format!("{path}.sha256"),
            format!("raw-byte SHA-256 mismatch: expected {expected_digest}, computed {actual_digest}"),
        ));
    }
    let json = serde_json::from_slice(&bytes)
        .map_err(|error| fail(&path, format!("artifact is not valid JSON: {error}")))?;
    Ok(RawArtifact {
        artifact_id,
        sha256: actual_digest,
        bytes,
        json,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Position {
    x_mm: i64,
    y_mm: i64,
    z_mm: i64,
}

fn parse_position(value: &Value, path: &str) -> Result<Position, String> {
    exact_keys(value, &["x", "y", "z"], path)?;
    Ok(Position {
        x_mm: int_field(value, "x", path)?,
        y_mm: int_field(value, "y", path)?,
        z_mm: int_field(value, "z", path)?,
    })
}

#[derive(Debug, Clone)]
struct Pose {
    sample_id: String,
    timestamp_ns: u64,
    position: Position,
}

struct Trajectory {
    trajectory_id: String,
    coordinate_frame_id: String,
    poses: Vec<Pose>,
}

fn parse_trajectory(value: &Value) -> Result<Trajectory, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "trajectory_id",
            "coordinate_frame_id",
            "samples",
        ],
        "$.artifacts.trajectory",
    )?;
    if string_field(value, "schema_version", "$.artifacts.trajectory")?
        != "wpt-canonical-trajectory-v0"
    {
        return Err(fail(
            "$.artifacts.trajectory.schema_version",
            "unsupported canonical trajectory schema",
        ));
    }
    let trajectory_id = string_field(value, "trajectory_id", "$.artifacts.trajectory")?.to_string();
    let coordinate_frame_id =
        string_field(value, "coordinate_frame_id", "$.artifacts.trajectory")?.to_string();
    let rows = array_field(value, "samples", "$.artifacts.trajectory")?;
    if !(2..=MAX_SAMPLES).contains(&rows.len()) {
        return Err(fail(
            "$.artifacts.trajectory.samples",
            format!("must contain 2..={MAX_SAMPLES} canonical poses"),
        ));
    }

    let mut poses = Vec::with_capacity(rows.len());
    let mut ids = BTreeSet::new();
    let mut previous_timestamp = None;
    for (index, row) in rows.iter().enumerate() {
        let path = format!("$.artifacts.trajectory.samples[{index}]");
        exact_keys(row, &["sample_id", "timestamp_ns", "position_mm"], &path)?;
        let sample_id = string_field(row, "sample_id", &path)?.to_string();
        if !ids.insert(sample_id.clone()) {
            return Err(fail(&path, format!("duplicate canonical sample_id {sample_id}")));
        }
        let timestamp_ns = uint_field(row, "timestamp_ns", &path)?;
        if previous_timestamp.is_some_and(|previous| timestamp_ns <= previous) {
            return Err(fail(
                &path,
                "canonical timestamps must be strictly increasing; samples are never silently sorted",
            ));
        }
        previous_timestamp = Some(timestamp_ns);
        let position = parse_position(
            field(row, "position_mm", &path)?,
            &format!("{path}.position_mm"),
        )?;
        poses.push(Pose {
            sample_id,
            timestamp_ns,
            position,
        });
    }
    Ok(Trajectory {
        trajectory_id,
        coordinate_frame_id,
        poses,
    })
}

#[derive(Debug, Clone)]
struct CoordinateTransform {
    transform_id: String,
    from_frame_id: String,
    to_frame_id: String,
    matrix_ppm: [[i64; 3]; 3],
    translation_mm: [i64; 3],
}

fn parse_transform(value: &Value, path: &str) -> Result<CoordinateTransform, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "transform_id",
            "from_frame_id",
            "to_frame_id",
            "matrix_ppm",
            "translation_mm",
        ],
        path,
    )?;
    if string_field(value, "schema_version", path)? != "wpt-coordinate-transform-v0" {
        return Err(fail(
            &format!("{path}.schema_version"),
            "unsupported coordinate transform schema",
        ));
    }
    let matrix_rows = array_field(value, "matrix_ppm", path)?;
    if matrix_rows.len() != 3 {
        return Err(fail(
            &format!("{path}.matrix_ppm"),
            "matrix must have exactly three rows",
        ));
    }
    let mut matrix = [[0_i64; 3]; 3];
    for (row_index, row) in matrix_rows.iter().enumerate() {
        let values = row
            .as_array()
            .ok_or_else(|| fail(&format!("{path}.matrix_ppm[{row_index}]"), "must be an array"))?;
        if values.len() != 3 {
            return Err(fail(
                &format!("{path}.matrix_ppm[{row_index}]"),
                "matrix row must have exactly three integer values",
            ));
        }
        for (column_index, value) in values.iter().enumerate() {
            let entry = value.as_i64().ok_or_else(|| {
                fail(
                    &format!("{path}.matrix_ppm[{row_index}][{column_index}]"),
                    "must be a signed integer",
                )
            })?;
            if !(-2_000_000..=2_000_000).contains(&entry) {
                return Err(fail(
                    &format!("{path}.matrix_ppm[{row_index}][{column_index}]"),
                    "coefficient exceeds the bounded v0 affine representation",
                ));
            }
            matrix[row_index][column_index] = entry;
        }
    }
    let translation = parse_position(
        field(value, "translation_mm", path)?,
        &format!("{path}.translation_mm"),
    )?;
    Ok(CoordinateTransform {
        transform_id: string_field(value, "transform_id", path)?.to_string(),
        from_frame_id: string_field(value, "from_frame_id", path)?.to_string(),
        to_frame_id: string_field(value, "to_frame_id", path)?.to_string(),
        matrix_ppm: matrix,
        translation_mm: [
            translation.x_mm,
            translation.y_mm,
            translation.z_mm,
        ],
    })
}

fn round_divide_ppm(value: i128) -> i128 {
    if value >= 0 {
        (value + PPM / 2) / PPM
    } else {
        (value - PPM / 2) / PPM
    }
}

fn transform_position(
    transform: &CoordinateTransform,
    source: &Position,
    path: &str,
) -> Result<Position, String> {
    let input = [
        source.x_mm as i128,
        source.y_mm as i128,
        source.z_mm as i128,
    ];
    let mut output = [0_i64; 3];
    for (axis, output_coordinate) in output.iter_mut().enumerate() {
        let mut sum = 0_i128;
        for (column, input_coordinate) in input.iter().enumerate() {
            let product = (transform.matrix_ppm[axis][column] as i128)
                .checked_mul(*input_coordinate)
                .ok_or_else(|| fail(path, "coordinate-transform multiplication overflowed"))?;
            sum = sum
                .checked_add(product)
                .ok_or_else(|| fail(path, "coordinate-transform sum overflowed"))?;
        }
        let transformed = round_divide_ppm(sum)
            .checked_add(transform.translation_mm[axis] as i128)
            .ok_or_else(|| fail(path, "coordinate-transform translation overflowed"))?;
        *output_coordinate = i64::try_from(transformed)
            .map_err(|_| fail(path, "transformed coordinate is outside i64 millimetres"))?;
    }
    Ok(Position {
        x_mm: output[0],
        y_mm: output[1],
        z_mm: output[2],
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReceipt {
    pub role: String,
    pub artifact_id: String,
    pub sha256: String,
    pub byte_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionVerification {
    pub projection_id: String,
    pub transform_id: String,
    pub sample_count: usize,
    pub coordinate_points_rederived: usize,
    pub maximum_clock_residual_ns: u64,
    pub maximum_clock_uncertainty_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryBundleVerificationReport {
    pub bundle_id: String,
    pub trajectory_id: String,
    pub canonical_sample_count: usize,
    pub em_projection: ProjectionVerification,
    pub pavement_projection: ProjectionVerification,
    pub artifact_receipts: Vec<ArtifactReceipt>,
    pub claim_ceiling: &'static str,
}

fn verify_projection(
    projection: &RawArtifact,
    transform_artifact: &RawArtifact,
    trajectory: &Trajectory,
    expected_schema: &str,
    global_maximum_clock_uncertainty_ns: u64,
    role: &str,
) -> Result<ProjectionVerification, String> {
    let path = format!("$.artifacts.{role}");
    let value = &projection.json;
    exact_keys(
        value,
        &[
            "schema_version",
            "projection_id",
            "trajectory_id",
            "coordinate_frame_id",
            "transform_id",
            "transform_sha256",
            "maximum_clock_uncertainty_ns",
            "samples",
        ],
        &path,
    )?;
    if string_field(value, "schema_version", &path)? != expected_schema {
        return Err(fail(
            &format!("{path}.schema_version"),
            "unsupported projection schema",
        ));
    }
    if string_field(value, "trajectory_id", &path)? != trajectory.trajectory_id {
        return Err(fail(
            &format!("{path}.trajectory_id"),
            "projection must bind the exact canonical trajectory_id",
        ));
    }
    let transform = parse_transform(
        &transform_artifact.json,
        &format!("$.artifacts.{}.json", transform_artifact.artifact_id),
    )?;
    let projection_transform_id = string_field(value, "transform_id", &path)?;
    if projection_transform_id != transform.transform_id {
        return Err(fail(
            &format!("{path}.transform_id"),
            "projection transform_id does not match the supplied transform artifact",
        ));
    }
    let projection_transform_sha = string_field(value, "transform_sha256", &path)?;
    if !valid_sha256(projection_transform_sha)
        || projection_transform_sha != transform_artifact.sha256
    {
        return Err(fail(
            &format!("{path}.transform_sha256"),
            "projection transform digest does not match the verified transform artifact",
        ));
    }
    if transform.from_frame_id != trajectory.coordinate_frame_id {
        return Err(fail(
            &format!("{path}.transform_id"),
            "coordinate transform source frame does not match canonical trajectory frame",
        ));
    }
    let projection_frame_id = string_field(value, "coordinate_frame_id", &path)?;
    if projection_frame_id != transform.to_frame_id {
        return Err(fail(
            &format!("{path}.coordinate_frame_id"),
            "projection frame does not match coordinate transform target frame",
        ));
    }
    let lane_maximum_uncertainty =
        uint_field(value, "maximum_clock_uncertainty_ns", &path)?;
    if lane_maximum_uncertainty > global_maximum_clock_uncertainty_ns {
        return Err(fail(
            &format!("{path}.maximum_clock_uncertainty_ns"),
            "lane clock-uncertainty maximum exceeds the bundle policy",
        ));
    }

    let rows = array_field(value, "samples", &path)?;
    if rows.len() != trajectory.poses.len() {
        return Err(fail(
            &format!("{path}.samples"),
            "projection must contain exactly one row per canonical pose",
        ));
    }
    let mut maximum_clock_residual_ns = 0_u64;
    let mut maximum_clock_uncertainty_ns = 0_u64;
    for (index, (row, pose)) in rows.iter().zip(trajectory.poses.iter()).enumerate() {
        let row_path = format!("{path}.samples[{index}]");
        let allowed_fields: &[&str] = if role == "em_projection" {
            &[
                "sample_id",
                "raw_timestamp_ns",
                "clock_correction_ns",
                "clock_uncertainty_ns",
                "canonical_timestamp_ns",
                "position_mm",
                "receiver_gap_um",
                "lateral_misalignment_um",
                "coupling_coefficient_ppm",
                "vehicle_dc_output_power_mw",
            ]
        } else {
            &[
                "sample_id",
                "raw_timestamp_ns",
                "clock_correction_ns",
                "clock_uncertainty_ns",
                "canonical_timestamp_ns",
                "position_mm",
                "interface_temperature_millidegrees_c",
                "tensile_strain_microstrain",
                "heat_flux_mw_per_m2",
            ]
        };
        exact_keys(row, allowed_fields, &row_path)?;
        if string_field(row, "sample_id", &row_path)? != pose.sample_id {
            return Err(fail(
                &format!("{row_path}.sample_id"),
                "projection sample identity/order differs from canonical trajectory",
            ));
        }
        let canonical_timestamp_ns = uint_field(row, "canonical_timestamp_ns", &row_path)?;
        if canonical_timestamp_ns != pose.timestamp_ns {
            return Err(fail(
                &format!("{row_path}.canonical_timestamp_ns"),
                "projection canonical timestamp must equal the referenced pose timestamp",
            ));
        }
        let raw_timestamp_ns = uint_field(row, "raw_timestamp_ns", &row_path)?;
        let clock_correction_ns = int_field(row, "clock_correction_ns", &row_path)?;
        let clock_uncertainty_ns = uint_field(row, "clock_uncertainty_ns", &row_path)?;
        if clock_uncertainty_ns > lane_maximum_uncertainty
            || clock_uncertainty_ns > global_maximum_clock_uncertainty_ns
        {
            return Err(fail(
                &format!("{row_path}.clock_uncertainty_ns"),
                "sample clock uncertainty exceeds declared bundle/lane policy",
            ));
        }
        let corrected_timestamp = (raw_timestamp_ns as i128)
            .checked_add(clock_correction_ns as i128)
            .ok_or_else(|| fail(&row_path, "corrected timestamp arithmetic overflowed"))?;
        if corrected_timestamp < 0 || corrected_timestamp > u64::MAX as i128 {
            return Err(fail(
                &row_path,
                "corrected timestamp is outside the unsigned nanosecond representation",
            ));
        }
        let residual = (corrected_timestamp - canonical_timestamp_ns as i128).abs() as u64;
        if residual > clock_uncertainty_ns {
            return Err(fail(
                &row_path,
                format!(
                    "clock residual {residual} ns exceeds declared uncertainty {clock_uncertainty_ns} ns"
                ),
            ));
        }
        maximum_clock_residual_ns = maximum_clock_residual_ns.max(residual);
        maximum_clock_uncertainty_ns = maximum_clock_uncertainty_ns.max(clock_uncertainty_ns);

        let observed_position = parse_position(
            field(row, "position_mm", &row_path)?,
            &format!("{row_path}.position_mm"),
        )?;
        let derived_position = transform_position(&transform, &pose.position, &row_path)?;
        if observed_position != derived_position {
            return Err(fail(
                &format!("{row_path}.position_mm"),
                format!(
                    "projection coordinate mismatch: expected ({},{},{}), found ({},{},{})",
                    derived_position.x_mm,
                    derived_position.y_mm,
                    derived_position.z_mm,
                    observed_position.x_mm,
                    observed_position.y_mm,
                    observed_position.z_mm
                ),
            ));
        }

        if role == "em_projection" {
            uint_field(row, "receiver_gap_um", &row_path)?;
            int_field(row, "lateral_misalignment_um", &row_path)?;
            let coupling = uint_field(row, "coupling_coefficient_ppm", &row_path)?;
            if coupling > 1_000_000 {
                return Err(fail(
                    &format!("{row_path}.coupling_coefficient_ppm"),
                    "coupling coefficient must be within 0..=1,000,000 ppm",
                ));
            }
            uint_field(row, "vehicle_dc_output_power_mw", &row_path)?;
        } else {
            int_field(row, "interface_temperature_millidegrees_c", &row_path)?;
            int_field(row, "tensile_strain_microstrain", &row_path)?;
            int_field(row, "heat_flux_mw_per_m2", &row_path)?;
        }
    }

    Ok(ProjectionVerification {
        projection_id: string_field(value, "projection_id", &path)?.to_string(),
        transform_id: transform.transform_id,
        sample_count: rows.len(),
        coordinate_points_rederived: rows.len(),
        maximum_clock_residual_ns,
        maximum_clock_uncertainty_ns,
    })
}

/// Verify raw artifact hashes and re-derive both projection coordinate streams.
///
/// The input contains the exact base64-encoded UTF-8 bytes received from each
/// source. Hashes are calculated over those exact bytes; no JSON reformatting or
/// key sorting is performed before hashing.
pub fn verify_bundle(value: &Value) -> Result<TrajectoryBundleVerificationReport, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "bundle_id",
            "evidence_kind",
            "quality_flags",
            "artifacts",
            "policy",
        ],
        "$",
    )?;
    if string_field(value, "schema_version", "$")? != "wpt-trajectory-artifact-bundle-v0" {
        return Err(fail("$.schema_version", "unsupported bundle schema"));
    }
    let bundle_id = string_field(value, "bundle_id", "$")?.to_string();
    if string_field(value, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth bundles only",
        ));
    }
    let flags = array_field(value, "quality_flags", "$")?;
    if flags.len() != 1 || flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 requires exactly SYNTHETIC_ONLY",
        ));
    }
    let policy = field(value, "policy", "$")?;
    exact_keys(
        policy,
        &[
            "maximum_bundle_artifact_bytes",
            "maximum_clock_uncertainty_ns",
        ],
        "$.policy",
    )?;
    let maximum_artifact_bytes =
        uint_field(policy, "maximum_bundle_artifact_bytes", "$.policy")?;
    if maximum_artifact_bytes == 0 {
        return Err(fail(
            "$.policy.maximum_bundle_artifact_bytes",
            "must be positive",
        ));
    }
    let maximum_clock_uncertainty_ns =
        uint_field(policy, "maximum_clock_uncertainty_ns", "$.policy")?;
    let artifacts = field(value, "artifacts", "$")?;
    exact_keys(artifacts, &REQUIRED_ARTIFACTS, "$.artifacts")?;

    let mut artifact_ids = BTreeSet::new();
    let mut total_bytes = 0_u64;
    let mut raw = std::collections::BTreeMap::new();
    for name in REQUIRED_ARTIFACTS {
        let artifact = read_artifact(
            artifacts,
            name,
            maximum_artifact_bytes,
            &mut artifact_ids,
        )?;
        total_bytes = total_bytes
            .checked_add(artifact.bytes.len() as u64)
            .ok_or_else(|| fail("$.artifacts", "total artifact byte count overflowed"))?;
        if total_bytes > maximum_artifact_bytes {
            return Err(fail(
                "$.artifacts",
                "combined artifact bytes exceed the declared bundle maximum",
            ));
        }
        raw.insert(name, artifact);
    }

    let trajectory_artifact = raw
        .get("trajectory")
        .ok_or_else(|| "missing verified trajectory artifact".to_string())?;
    let trajectory = parse_trajectory(&trajectory_artifact.json)?;
    let em = verify_projection(
        raw.get("em_projection")
            .ok_or_else(|| "missing EM projection artifact".to_string())?,
        raw.get("em_transform")
            .ok_or_else(|| "missing EM transform artifact".to_string())?,
        &trajectory,
        "wpt-em-trajectory-projection-v0",
        maximum_clock_uncertainty_ns,
        "em_projection",
    )?;
    let pavement = verify_projection(
        raw.get("pavement_projection")
            .ok_or_else(|| "missing pavement projection artifact".to_string())?,
        raw.get("pavement_transform")
            .ok_or_else(|| "missing pavement transform artifact".to_string())?,
        &trajectory,
        "wpt-pavement-trajectory-projection-v0",
        maximum_clock_uncertainty_ns,
        "pavement_projection",
    )?;

    let artifact_receipts = REQUIRED_ARTIFACTS
        .iter()
        .map(|name| {
            let artifact = &raw[name];
            ArtifactReceipt {
                role: (*name).to_string(),
                artifact_id: artifact.artifact_id.clone(),
                sha256: artifact.sha256.clone(),
                byte_length: artifact.bytes.len(),
            }
        })
        .collect();

    Ok(TrajectoryBundleVerificationReport {
        bundle_id,
        trajectory_id: trajectory.trajectory_id,
        canonical_sample_count: trajectory.poses.len(),
        em_projection: em,
        pavement_projection: pavement,
        artifact_receipts,
        claim_ceiling: "Exact raw-byte digest, sample lineage/order, bounded clock-correction and affine coordinate replay for a synthetic bundle only; no physical coupling, structural response, thermal constitutive, fatigue, measurement authenticity, safety or field-performance claim.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-trajectory-artifact-verifier-v0/bundle.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic WPT trajectory artifact bundle JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    fn rewrite_artifact(
        bundle: &mut Value,
        role: &str,
        mutate: impl FnOnce(&mut Value),
    ) -> String {
        let encoded = bundle["artifacts"][role]["bytes_base64"].as_str().unwrap();
        let bytes = STANDARD.decode(encoded).unwrap();
        let mut json: Value = serde_json::from_slice(&bytes).unwrap();
        mutate(&mut json);
        let bytes = serde_json::to_vec(&json).unwrap();
        let digest = sha256_hex(&bytes);
        bundle["artifacts"][role]["sha256"] = Value::String(digest.clone());
        bundle["artifacts"][role]["bytes_base64"] = Value::String(STANDARD.encode(bytes));
        digest
    }

    #[test]
    fn known_truth_verifies_all_five_raw_digests_clocks_and_transforms() {
        let report = verify_bundle(&fixture()).unwrap();
        assert_eq!(report.bundle_id, "bundle-known-truth-01");
        assert_eq!(report.trajectory_id, "trajectory-known-truth-01");
        assert_eq!(report.canonical_sample_count, 2);
        assert_eq!(report.artifact_receipts.len(), 5);
        assert_eq!(report.em_projection.sample_count, 2);
        assert_eq!(report.pavement_projection.sample_count, 2);
        assert_eq!(report.em_projection.coordinate_points_rederived, 2);
        assert_eq!(report.pavement_projection.coordinate_points_rederived, 2);
        assert_eq!(report.em_projection.maximum_clock_residual_ns, 0);
        assert_eq!(report.pavement_projection.maximum_clock_residual_ns, 0);
        assert_eq!(report.em_projection.maximum_clock_uncertainty_ns, 200);
        assert_eq!(report.pavement_projection.maximum_clock_uncertainty_ns, 150);
    }

    #[test]
    fn changed_raw_bytes_without_new_digest_are_rejected() {
        let mut bundle = fixture();
        let encoded = bundle["artifacts"]["trajectory"]["bytes_base64"]
            .as_str()
            .unwrap()
            .to_string();
        let mut bytes = STANDARD.decode(encoded).unwrap();
        bytes[0] ^= 1;
        bundle["artifacts"]["trajectory"]["bytes_base64"] =
            Value::String(STANDARD.encode(bytes));
        assert!(verify_bundle(&bundle).unwrap_err().contains("SHA-256 mismatch"));
    }

    #[test]
    fn clock_residual_outside_measurement_uncertainty_is_rejected() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "em_projection", |projection| {
            projection["samples"][0]["raw_timestamp_ns"] = Value::from(1_000_000_400_u64);
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("exceeds declared uncertainty")
        );
    }

    #[test]
    fn projection_cannot_relabel_canonical_pose_time() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "pavement_projection", |projection| {
            projection["samples"][0]["canonical_timestamp_ns"] =
                Value::from(1_000_000_001_u64);
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("must equal the referenced pose timestamp")
        );
    }

    #[test]
    fn projection_coordinate_must_be_rederived_from_verified_transform_bytes() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "em_projection", |projection| {
            projection["samples"][1]["position_mm"]["y"] = Value::from(16);
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("projection coordinate mismatch")
        );
    }

    #[test]
    fn transform_digest_receipt_must_match_the_actual_transform_artifact() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "em_transform", |transform| {
            transform["translation_mm"]["x"] = Value::from(11);
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("does not match the verified transform artifact")
        );
    }

    #[test]
    fn projection_sample_order_must_match_canonical_pose_order() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "em_projection", |projection| {
            projection["samples"].as_array_mut().unwrap().reverse();
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("identity/order differs")
        );
    }

    #[test]
    fn duplicate_canonical_sample_identity_is_rejected() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "trajectory", |trajectory| {
            trajectory["samples"][1]["sample_id"] = Value::String("pose-000".into());
        });
        assert!(verify_bundle(&bundle).unwrap_err().contains("duplicate canonical sample_id"));
    }

    #[test]
    fn excessive_reported_clock_uncertainty_is_not_used_to_hide_residual() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "pavement_projection", |projection| {
            projection["samples"][0]["clock_uncertainty_ns"] = Value::from(600);
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("exceeds declared bundle/lane policy")
        );
    }

    #[test]
    fn wrong_projection_trajectory_id_is_rejected() {
        let mut bundle = fixture();
        rewrite_artifact(&mut bundle, "pavement_projection", |projection| {
            projection["trajectory_id"] = Value::String("other-trajectory".into());
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("exact canonical trajectory_id")
        );
    }

    #[test]
    fn unknown_properties_and_non_synthetic_evidence_fail_closed() {
        let mut bundle = fixture();
        bundle["debug_override"] = Value::Bool(true);
        assert!(verify_bundle(&bundle).unwrap_err().contains("unknown property"));

        let mut bundle = fixture();
        bundle["evidence_kind"] = Value::String("MEASURED".into());
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("synthetic known-truth")
        );
    }

    #[test]
    fn malformed_base64_and_invalid_transform_dimensions_fail_closed() {
        let mut bundle = fixture();
        bundle["artifacts"]["trajectory"]["bytes_base64"] =
            Value::String("%%%not-base64%%%".into());
        assert!(verify_bundle(&bundle).unwrap_err().contains("bytes_base64"));

        let mut bundle = fixture();
        let transform_digest = rewrite_artifact(&mut bundle, "em_transform", |transform| {
            transform["matrix_ppm"] = Value::Array(vec![Value::Array(vec![Value::from(1)])]);
        });
        rewrite_artifact(&mut bundle, "em_projection", |projection| {
            projection["transform_sha256"] = Value::String(transform_digest);
        });
        assert!(
            verify_bundle(&bundle)
                .unwrap_err()
                .contains("matrix must have exactly three rows")
        );
    }
}
