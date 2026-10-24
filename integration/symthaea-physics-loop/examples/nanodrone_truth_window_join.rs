//! Attach exact multi-horizon ground-truth windows from a raw trajectory to a per-run
//! prediction CSV. This enables scoring every prediction window without reconstructing
//! future states by shifting a concatenated one-step truth column.
//!
//! The caller must provide denormalized SI-valued predictions. The required CLI flag is
//! an explicit assertion of that input contract; it is recorded but cannot be proven from
//! a CSV alone. Run each independent trajectory file separately.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const HORIZON: usize = 50;
const EXPECTED_DT_S: f64 = 0.010;
const MAX_DT_DEVIATION_S: f64 = 0.002;
const QUATERNION_NORM_MIN: f64 = 0.95;
const QUATERNION_NORM_MAX: f64 = 1.05;
const STATE_ALIGNMENT_ATOL: f64 = 5.0e-3;
const ORIENTATION_ALIGNMENT_ATOL_RAD: f64 = 1.0e-2;
const COMMON_FIELDS: [&str; 9] = ["x", "y", "z", "vx", "vy", "vz", "wx", "wy", "wz"];
const QUATERNION_FIELDS: [&str; 4] = ["qx", "qy", "qz", "qw"];
const ROTVEC_FIELDS: [&str; 3] = ["rx", "ry", "rz"];
const MOTOR_INPUT_FIELDS: [&str; 4] = ["m1_rads", "m2_rads", "m3_rads", "m4_rads"];
const MOTOR_INPUT_ALIGNMENT_ATOL: f64 = 1.0e-3;
const RAW_REQUIRED: [&str; 18] = [
    "t", "x", "y", "z", "vx", "vy", "vz", "qx", "qy", "qz", "qw", "wx", "wy", "wz",
    "m1_rads", "m2_rads", "m3_rads", "m4_rads",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrientationMode {
    Quaternion,
    RotationVector,
}

impl OrientationMode {
    fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Quaternion => &QUATERNION_FIELDS,
            Self::RotationVector => &ROTVEC_FIELDS,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Quaternion => "xyzw_quaternion",
            Self::RotationVector => "so3_log_rotation_vector",
        }
    }
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn name_index(headers: &StringRecord, label: &str) -> Result<HashMap<String, usize>, io::Error> {
    let mut indices = HashMap::new();
    for (index, name) in headers.iter().enumerate() {
        if name.trim().is_empty() {
            return Err(invalid_input(format!("{label}: blank header at column {index}")));
        }
        if indices.insert(name.to_owned(), index).is_some() {
            return Err(invalid_input(format!("{label}: duplicate header '{name}'")));
        }
    }
    Ok(indices)
}

fn parse_f64(raw: &str, label: &str) -> Result<f64, io::Error> {
    let value = raw.parse::<f64>().map_err(|error| {
        invalid_input(format!("{label}: expected a numeric value ({error})"))
    })?;
    if !value.is_finite() {
        return Err(invalid_input(format!("{label}: value must be finite")));
    }
    Ok(value)
}

fn row_value(
    record: &StringRecord,
    indices: &HashMap<String, usize>,
    column: &str,
    row_number: usize,
    label: &str,
) -> Result<f64, io::Error> {
    let index = *indices
        .get(column)
        .ok_or_else(|| invalid_input(format!("{label}: missing required column '{column}'")))?;
    let raw = record.get(index).ok_or_else(|| {
        invalid_input(format!("{label}: row {row_number} is missing column '{column}'"))
    })?;
    parse_f64(raw, &format!("{label}: row {row_number} column '{column}'"))
}

fn check_dt(dt: f64, label: &str, row_number: usize) -> Result<(), io::Error> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err(invalid_input(format!(
            "{label}: row {row_number} timestamp is not strictly increasing (dt={dt})"
        )));
    }
    let deviation = (dt - EXPECTED_DT_S).abs();
    if deviation > MAX_DT_DEVIATION_S {
        return Err(invalid_input(format!(
            "{label}: row {row_number} dt={dt:.9}s differs from 100 Hz by {deviation:.9}s (limit {MAX_DT_DEVIATION_S:.9}s)"
        )));
    }
    Ok(())
}

fn quaternion_distance_sign_invariant(a: [f64; 4], b: [f64; 4]) -> f64 {
    let norm_a = a.iter().map(|value| value * value).sum::<f64>().sqrt();
    let norm_b = b.iter().map(|value| value * value).sum::<f64>().sqrt();
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f64>() / (norm_a * norm_b);
    2.0 * dot.abs().clamp(0.0, 1.0).acos()
}

fn check_quaternion(q: [f64; 4], label: &str) -> Result<(), io::Error> {
    let norm = q.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || !(QUATERNION_NORM_MIN..=QUATERNION_NORM_MAX).contains(&norm) {
        return Err(invalid_input(format!(
            "{label}: quaternion norm {norm} outside [{QUATERNION_NORM_MIN}, {QUATERNION_NORM_MAX}]"
        )));
    }
    Ok(())
}

/// Matches PyTorch3D's real-first quaternion_to_axis_angle convention after its public
/// loader reorders XYZW to WXYZ. We intentionally do not canonicalize quaternion sign,
/// since this is a data conversion used to match the upstream rotation-vector pipeline.
fn quaternion_to_rotvec(q: [f64; 4]) -> [f64; 3] {
    let imaginary_norm = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
    if imaginary_norm <= f64::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    let angle = 2.0 * imaginary_norm.atan2(q[3]);
    let scale = angle / imaginary_norm;
    [q[0] * scale, q[1] * scale, q[2] * scale]
}

/// Reverse the public benchmark's SO(3)-log representation back to XYZW. In quaternion
/// mode, future targets pass through the same q -> axis-angle -> q round trip as the
/// upstream result table, rather than mixing raw measured quaternions with reconstructed
/// model-table quaternions.
fn rotvec_to_quaternion(r: [f64; 3]) -> [f64; 4] {
    let angle = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    if angle <= f64::EPSILON {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let scale = (angle * 0.5).sin() / angle;
    [r[0] * scale, r[1] * scale, r[2] * scale, (angle * 0.5).cos()]
}

fn run_id_from_path(path: &Path, label: &str) -> Result<String, io::Error> {
    let filename = path.file_name().and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input(format!("{label}: filename must be UTF-8")))?;
    if path.extension().and_then(|ext| ext.to_str()) != Some("csv") {
        return Err(invalid_input(format!("{label}: '{filename}' must end in .csv")));
    }
    let stem = path.file_stem().and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input(format!("{label}: file stem must be UTF-8")))?
        .to_ascii_lowercase();
    let parts = stem.split('_').collect::<Vec<_>>();
    if parts.first().copied() != Some("melon") {
        return Err(invalid_input(format!("{label}: '{filename}' must begin with 'melon'")));
    }
    let run_id = parts.last().copied().unwrap_or_default();
    if !["run1", "run2", "run3"].contains(&run_id) {
        return Err(invalid_input(format!(
            "{label}: '{filename}' must end in exactly run1, run2, or run3"
        )));
    }
    Ok(run_id.to_owned())
}

fn fields_for_mode(mode: OrientationMode) -> Vec<&'static str> {
    let mut fields = COMMON_FIELDS.to_vec();
    fields.extend_from_slice(mode.fields());
    fields
}

fn target_key(field: &str, horizon: usize) -> String {
    format!("{field}_true_h{horizon}")
}

fn prediction_key(field: &str, horizon: usize) -> String {
    format!("{field}_pred_h{horizon}")
}

fn input_state_key(field: &str) -> String {
    format!("x0_{field}")
}

fn motor_sequence_key(field: &str, input_step: usize) -> String {
    format!("{field}_input_t{input_step}")
}

fn motor_sequence_time_key(input_step: usize) -> String {
    format!("input_time_t{input_step}")
}

fn input_state_value(row: &HashMap<String, f64>, field: &str) -> f64 {
    match field {
        "rx" | "ry" | "rz" => {
            let rotation = quaternion_to_rotvec([row["qx"], row["qy"], row["qz"], row["qw"]]);
            match field {
                "rx" => rotation[0],
                "ry" => rotation[1],
                _ => rotation[2],
            }
        }
        _ => row[field],
    }
}

fn detect_prediction_mode(indices: &HashMap<String, usize>, label: &str) -> Result<OrientationMode, io::Error> {
    let has_quaternion = QUATERNION_FIELDS.iter().all(|field| {
        indices.contains_key(*field) && (1..=HORIZON).all(|h| indices.contains_key(&prediction_key(field, h)))
    });
    let has_rotvec = ROTVEC_FIELDS.iter().all(|field| {
        indices.contains_key(*field) && (1..=HORIZON).all(|h| indices.contains_key(&prediction_key(field, h)))
    });
    if has_quaternion {
        Ok(OrientationMode::Quaternion)
    } else if has_rotvec {
        Ok(OrientationMode::RotationVector)
    } else {
        Err(invalid_input(format!(
            "{label}: expected complete true and horizon 1..={HORIZON} quaternion or rotation-vector fields"
        )))
    }
}

#[derive(Debug)]
struct RawTrajectory {
    rows: Vec<HashMap<String, f64>>,
    sha256: String,
    filename: String,
}

fn load_raw(path: &Path) -> Result<RawTrajectory, Box<dyn Error>> {
    let filename = path.file_name().and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input("raw trajectory filename must be UTF-8"))?.to_owned();
    let run_id = run_id_from_path(path, "raw trajectory")?;
    let bytes = fs::read(path)?;
    if bytes.is_empty() {
        return Err(invalid_input("raw trajectory is empty").into());
    }
    let digest = sha256(&bytes);
    let mut reader = ReaderBuilder::new().has_headers(true).flexible(false).from_reader(bytes.as_slice());
    let headers = reader.headers()?.clone();
    let indices = name_index(&headers, "raw trajectory")?;
    for required in RAW_REQUIRED {
        if !indices.contains_key(required) {
            return Err(invalid_input(format!("raw trajectory: missing '{required}'")).into());
        }
    }

    let mut rows = Vec::new();
    let mut previous_time = None::<f64>;
    for (index, record_result) in reader.records().enumerate() {
        let record = record_result?;
        let row_number = index + 2;
        let mut row = HashMap::new();
        for column in RAW_REQUIRED {
            row.insert(column.to_owned(), row_value(&record, &indices, column, row_number, "raw trajectory")?);
        }
        let timestamp = row["t"];
        if let Some(previous) = previous_time {
            check_dt(timestamp - previous, "raw trajectory", row_number)?;
        }
        previous_time = Some(timestamp);
        check_quaternion(
            [row["qx"], row["qy"], row["qz"], row["qw"]],
            &format!("raw trajectory row {row_number}"),
        )?;
        for motor in ["m1_rads", "m2_rads", "m3_rads", "m4_rads"] {
            if row[motor] < 0.0 {
                return Err(invalid_input(format!("raw trajectory row {row_number}: {motor} must be non-negative")).into());
            }
        }
        rows.push(row);
    }
    if rows.len() <= HORIZON {
        return Err(invalid_input(format!("raw trajectory needs more than {HORIZON} rows")).into());
    }
    Ok(RawTrajectory { rows, sha256: digest, filename })
}

fn join_files_internal(
    raw_path: &Path,
    prediction_path: &Path,
    output_path: &Path,
    asserted_si_units: bool,
    prediction_row_offset: usize,
    pooled_prediction: bool,
) -> Result<Value, Box<dyn Error>> {
    if !asserted_si_units {
        return Err(invalid_input("must explicitly acknowledge denormalized SI-unit inputs with --state-units-si").into());
    }
    let raw_run_id = run_id_from_path(raw_path, "raw trajectory")?;
    let prediction_run_id = if pooled_prediction {
        raw_run_id.clone()
    } else {
        run_id_from_path(prediction_path, "prediction table")?
    };
    let output_run_id = run_id_from_path(output_path, "output table")?;
    if output_run_id != raw_run_id {
        return Err(invalid_input(format!(
            "output must retain the source trajectory/run identity: expected {raw_run_id}, got {output_run_id}"
        )).into());
    }
    let raw_canonical = raw_path.canonicalize()?;
    let prediction_canonical = prediction_path.canonicalize()?;
    if raw_canonical == prediction_canonical {
        return Err(invalid_input("raw trajectory and prediction inputs must be different files").into());
    }
    if output_path.exists() {
        let output_canonical = output_path.canonicalize()?;
        if output_canonical == raw_canonical || output_canonical == prediction_canonical {
            return Err(invalid_input("output must not overwrite a raw trajectory or prediction input").into());
        }
    }
    if raw_run_id != prediction_run_id {
        return Err(invalid_input(format!(
            "run mismatch: raw data is {raw_run_id}, predictions are {prediction_run_id}"
        )).into());
    }

    let raw = load_raw(raw_path)?;
    let prediction_bytes = fs::read(prediction_path)?;
    if prediction_bytes.is_empty() {
        return Err(invalid_input("prediction table is empty").into());
    }
    let prediction_sha256 = sha256(&prediction_bytes);
    let mut reader = ReaderBuilder::new().has_headers(true).flexible(false).from_reader(prediction_bytes.as_slice());
    let prediction_headers = reader.headers()?.clone();
    let prediction_indices = name_index(&prediction_headers, "prediction table")?;
    let mode = detect_prediction_mode(&prediction_indices, "prediction table")?;
    let motor_input_count = MOTOR_INPUT_FIELDS.iter()
        .filter(|field| prediction_indices.contains_key(**field))
        .count();
    if motor_input_count != 0 && motor_input_count != MOTOR_INPUT_FIELDS.len() {
        return Err(invalid_input(format!(
            "prediction table has a partial motor-input schema ({motor_input_count}/{} columns); provide all four motor inputs or none",
            MOTOR_INPUT_FIELDS.len()
        )).into());
    }
    let prediction_has_motor_inputs = motor_input_count == MOTOR_INPUT_FIELDS.len();

    let mut base_state_fields = fields_for_mode(mode);
    base_state_fields.push("t");
    for field in &base_state_fields {
        if !prediction_indices.contains_key(*field) {
            return Err(invalid_input(format!("prediction table: missing base column '{field}'")).into());
        }
    }
    for field in fields_for_mode(mode) {
        for horizon in 1..=HORIZON {
            let key = prediction_key(field, horizon);
            if !prediction_indices.contains_key(&key) {
                return Err(invalid_input(format!("prediction table: missing '{key}'")).into());
            }
        }
    }

    let mut prediction_rows = Vec::new();
    let mut previous_prediction_time = None::<f64>;
    for (index, record_result) in reader.records().enumerate() {
        let record = record_result?;
        let row_number = index + 2;
        let timestamp = row_value(&record, &prediction_indices, "t", row_number, "prediction table")?;
        if !pooled_prediction {
            if let Some(previous) = previous_prediction_time {
                check_dt(timestamp - previous, "prediction table", row_number)?;
            }
            previous_prediction_time = Some(timestamp);
        }

        for field in fields_for_mode(mode) {
            row_value(&record, &prediction_indices, field, row_number, "prediction table")?;
        }
        for field in fields_for_mode(mode) {
            for horizon in 1..=HORIZON {
                row_value(
                    &record,
                    &prediction_indices,
                    &prediction_key(field, horizon),
                    row_number,
                    "prediction table",
                )?;
            }
        }

        // Validate quaternion representations for every row at ingress. Row-to-raw
        // alignment is checked only after the pooled table has been sliced into runs.
        if mode == OrientationMode::Quaternion {
            check_quaternion([
                row_value(&record, &prediction_indices, "qx", row_number, "prediction table")?,
                row_value(&record, &prediction_indices, "qy", row_number, "prediction table")?,
                row_value(&record, &prediction_indices, "qz", row_number, "prediction table")?,
                row_value(&record, &prediction_indices, "qw", row_number, "prediction table")?,
            ], &format!("prediction table row {row_number} base orientation"))?;
            for horizon in 1..=HORIZON {
                check_quaternion([
                    row_value(&record, &prediction_indices, &prediction_key("qx", horizon), row_number, "prediction table")?,
                    row_value(&record, &prediction_indices, &prediction_key("qy", horizon), row_number, "prediction table")?,
                    row_value(&record, &prediction_indices, &prediction_key("qz", horizon), row_number, "prediction table")?,
                    row_value(&record, &prediction_indices, &prediction_key("qw", horizon), row_number, "prediction table")?,
                ], &format!("prediction table row {row_number} prediction h={horizon}"))?;
            }
        }
        prediction_rows.push(record);
    }
    let expected_rows = raw.rows.len() - HORIZON;
    let slice_end = prediction_row_offset.checked_add(expected_rows)
        .ok_or_else(|| invalid_input("prediction row offset overflows usize"))?;
    if (!pooled_prediction && (prediction_row_offset != 0 || prediction_rows.len() != expected_rows))
        || (pooled_prediction && slice_end > prediction_rows.len())
    {
        return Err(invalid_input(format!(
            "row alignment failed: raw run has {} rows and needs {} prediction rows at offset {}, but the prediction table has {} rows",
            raw.rows.len(), expected_rows, prediction_row_offset, prediction_rows.len()
        )).into());
    }
    let segment_rows = &prediction_rows[prediction_row_offset..slice_end];

    // Each slice has an independent raw-state index space. The upstream result table's
    // base truth is x_(i+1), while horizon h targets raw row i+h; validate only after the
    // pooled row segment has been assigned to its source run.
    for (index, record) in segment_rows.iter().enumerate() {
        let row_number = index + 2;
        let one_step_truth = &raw.rows[index + 1];
        for field in COMMON_FIELDS {
            let observed = row_value(record, &prediction_indices, field, row_number, "prediction table run slice")?;
            let expected = one_step_truth[field];
            if (observed - expected).abs() > STATE_ALIGNMENT_ATOL {
                return Err(invalid_input(format!(
                    "row alignment failed for {raw_run_id} at prediction row {row_number}: base truth '{field}'={observed} does not match raw row {} value {expected} within {STATE_ALIGNMENT_ATOL}",
                    index + 3
                )).into());
            }
        }

        let raw_q = [
            one_step_truth["qx"], one_step_truth["qy"],
            one_step_truth["qz"], one_step_truth["qw"],
        ];
        if mode == OrientationMode::Quaternion {
            let observed_q = [
                row_value(record, &prediction_indices, "qx", row_number, "prediction table run slice")?,
                row_value(record, &prediction_indices, "qy", row_number, "prediction table run slice")?,
                row_value(record, &prediction_indices, "qz", row_number, "prediction table run slice")?,
                row_value(record, &prediction_indices, "qw", row_number, "prediction table run slice")?,
            ];
            let orientation_error = quaternion_distance_sign_invariant(raw_q, observed_q);
            if orientation_error > ORIENTATION_ALIGNMENT_ATOL_RAD {
                return Err(invalid_input(format!(
                    "row alignment failed for {raw_run_id} at prediction row {row_number}: base orientation error {orientation_error} rad exceeds {ORIENTATION_ALIGNMENT_ATOL_RAD}"
                )).into());
            }
        } else {
            let raw_rotvec = quaternion_to_rotvec(raw_q);
            for (field, expected) in ["rx", "ry", "rz"].iter().zip(raw_rotvec) {
                let observed = row_value(record, &prediction_indices, field, row_number, "prediction table run slice")?;
                if (observed - expected).abs() > ORIENTATION_ALIGNMENT_ATOL_RAD {
                    return Err(invalid_input(format!(
                        "row alignment failed for {raw_run_id} at prediction row {row_number}: base orientation '{field}'={observed} does not match raw row {} value {expected}",
                        index + 3
                    )).into());
                }
            }
        }

        // A transition window i uses motor inputs recorded at raw row i. If upstream
        // output already carries these channels, verify them against raw data; otherwise
        // attach them before writing so comparisons can prove identical conditioning.
        for field in MOTOR_INPUT_FIELDS {
            if prediction_has_motor_inputs {
                let observed = row_value(record, &prediction_indices, field, row_number, "prediction table run slice")?;
                let expected = raw.rows[index][field];
                if (observed - expected).abs() > MOTOR_INPUT_ALIGNMENT_ATOL {
                    return Err(invalid_input(format!(
                        "motor-input alignment failed for {raw_run_id} at prediction row {row_number}: '{field}'={observed} does not match raw row {} value {expected} within {MOTOR_INPUT_ALIGNMENT_ATOL}",
                        index + 2
                    )).into());
                }
            }
        }
    }

    for index in 1..segment_rows.len() {
        let before = row_value(&segment_rows[index - 1], &prediction_indices, "t", index + prediction_row_offset + 1, "prediction table")?;
        let after = row_value(&segment_rows[index], &prediction_indices, "t", index + prediction_row_offset + 2, "prediction table")?;
        check_dt(after - before, "prediction table run slice", index + 2)?;
    }

    let target_columns = fields_for_mode(mode).into_iter()
        .flat_map(|field| (1..=HORIZON).map(move |horizon| target_key(field, horizon)))
        .collect::<Vec<_>>();
    let mut output_headers = prediction_headers.clone();
    if !prediction_has_motor_inputs {
        for field in MOTOR_INPUT_FIELDS {
            output_headers.push_field(field);
        }
    }

    let mut conditioning_columns = vec!["x0_t".to_owned()];
    conditioning_columns.extend(
        fields_for_mode(mode).into_iter().map(input_state_key),
    );
    conditioning_columns.extend((0..HORIZON).map(motor_sequence_time_key));
    conditioning_columns.extend(
        MOTOR_INPUT_FIELDS.iter().flat_map(|field|
            (0..HORIZON).map(move |step| motor_sequence_key(field, step))
        ),
    );

    let mut existing = output_headers.iter().map(str::to_owned).collect::<HashSet<_>>();
    for column in &conditioning_columns {
        if existing.contains(column) {
            return Err(invalid_input(format!(
                "prediction table already contains conditioning column '{column}'; refusing to overwrite or mix provenance"
            )).into());
        }
        output_headers.push_field(column);
        existing.insert(column.clone());
    }
    for target in &target_columns {
        if existing.contains(target) {
            return Err(invalid_input(format!("prediction table already contains target column '{target}'")).into());
        }
        output_headers.push_field(target);
    }

    let output_tmp = PathBuf::from(format!("{}.tmp", output_path.display()));
    if let Some(parent) = output_tmp.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let write_result: Result<String, Box<dyn Error>> = (|| {
        let file = File::create(&output_tmp)?;
        let mut writer = WriterBuilder::new().has_headers(false).from_writer(file);
        writer.write_record(&output_headers)?;
        for (window_index, prediction_record) in segment_rows.iter().enumerate() {
            let mut out = prediction_record.clone();
            if !prediction_has_motor_inputs {
                for field in MOTOR_INPUT_FIELDS {
                    out.push_field(&format!("{:.17}", raw.rows[window_index][field]));
                }
            }

            // Match the real model contract: x0 is raw state i and u_seq contains
            // the 50 motor rows i..i+49, while target h is raw state i+h.
            let initial_state = &raw.rows[window_index];
            out.push_field(&format!("{:.17}", initial_state["t"]));
            for field in fields_for_mode(mode) {
                out.push_field(&format!("{:.17}", input_state_value(initial_state, field)));
            }
            for input_step in 0..HORIZON {
                out.push_field(&format!("{:.17}", raw.rows[window_index + input_step]["t"]));
            }
            for field in MOTOR_INPUT_FIELDS {
                for input_step in 0..HORIZON {
                    let value = raw.rows[window_index + input_step][field];
                    out.push_field(&format!("{value:.17}"));
                }
            }

            for horizon in 1..=HORIZON {
                // The upstream QuadDataset window i predicts x_(i+1) ... x_(i+H).
                // Therefore horizon h maps to raw trajectory row i+h.
                let truth = &raw.rows[window_index + horizon];
                for field in fields_for_mode(mode) {
                    let value = match field {
                        "rx" | "ry" | "rz" => {
                            let q = [truth["qx"], truth["qy"], truth["qz"], truth["qw"]];
                            let rotation = quaternion_to_rotvec(q);
                            match field {
                                "rx" => rotation[0],
                                "ry" => rotation[1],
                                _ => rotation[2],
                            }
                        }
                        "qx" | "qy" | "qz" | "qw" => {
                            let q = [truth["qx"], truth["qy"], truth["qz"], truth["qw"]];
                            let rotation = quaternion_to_rotvec(q);
                            let reconstructed = rotvec_to_quaternion(rotation);
                            match field {
                                "qx" => reconstructed[0],
                                "qy" => reconstructed[1],
                                "qz" => reconstructed[2],
                                _ => reconstructed[3],
                            }
                        }
                        _ => truth[field],
                    };
                    out.push_field(&format!("{value:.17}"));
                }
            }
            writer.write_record(&out)?;
        }
        writer.flush()?;
        drop(writer);
        fs::rename(&output_tmp, output_path)?;
        let bytes = fs::read(output_path)?;
        Ok(sha256(&bytes))
    })();
    let output_sha256 = match write_result {
        Ok(digest) => digest,
        Err(error) => {
            let _ = fs::remove_file(&output_tmp);
            return Err(error);
        }
    };

    Ok(json!({
        "benchmark_id": "symtropy-nanodrone-full-window-truth-join-v0.1",
        "claim_ceiling": "row_index_aligned_ground_truth_attachment_only_no_model_or_safety_claim",
        "source": {
            "dataset_file": raw.filename,
            "dataset_sha256": raw.sha256,
            "prediction_file": prediction_path.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
            "prediction_sha256": prediction_sha256,
            "output_file": output_path.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
            "output_sha256": output_sha256
        },
        "split_identity": {
            "trajectory": "melon",
            "run_id": raw_run_id,
            "one_file_per_run": true,
            "run_boundary_crossing": false
        },
        "design": {
            "source_row_count": raw.rows.len(),
            "prediction_window_count": segment_rows.len(),
            "prediction_row_offset_in_source_table": prediction_row_offset,
            "pooled_prediction_source": pooled_prediction,
            "horizon_steps": HORIZON,
            "sample_period_s": EXPECTED_DT_S,
            "prediction_state_units": "caller_asserted_denormalized_SI_units",
            "orientation_representation": mode.label(),
            "motor_input_columns_present": true,
            "conditioning_state_columns": fields_for_mode(mode).into_iter().map(input_state_key).collect::<Vec<_>>(),
            "motor_input_sequence_columns": MOTOR_INPUT_FIELDS.iter()
                .flat_map(|field| (0..HORIZON).map(move |step| motor_sequence_key(field, step)))
                .collect::<Vec<_>>(),
            "motor_input_timestamp_columns": (0..HORIZON).map(motor_sequence_time_key).collect::<Vec<_>>(),
            "motor_input_alignment": "the complete four-channel motor input sequence and each sample timestamp are attached from raw rows i..i+49; pre-existing one-step motor columns, when supplied, are verified against raw row i",
            "raw_state_to_prediction_row_alignment": "prediction row i is window starting at raw row i; x0 is raw state i; motor inputs are raw rows i..i+49; true state at horizon h is raw row i+h",
            "all_available_prediction_windows_scored_at_each_horizon": true,
            "target_columns_written": target_columns.len(),
            "column_name_mapping": "append {state}_true_h{horizon} columns; preserve all prediction columns and rows"
        }
    }))
}

fn join_files(
    raw_path: &Path,
    prediction_path: &Path,
    output_path: &Path,
    asserted_si_units: bool,
) -> Result<Value, Box<dyn Error>> {
    join_files_internal(raw_path, prediction_path, output_path, asserted_si_units, 0, false)
}

fn join_pooled_predictions(
    prediction_path: &Path,
    raw_paths: &[PathBuf],
    output_dir: &Path,
    asserted_si_units: bool,
) -> Result<Value, Box<dyn Error>> {
    if !asserted_si_units {
        return Err(invalid_input("must explicitly acknowledge denormalized SI-unit inputs with --state-units-si").into());
    }
    if raw_paths.len() != 3 {
        return Err(invalid_input("pooled mode requires exactly Melon run1, run2, and run3 raw trajectories").into());
    }

    let mut loaded = Vec::new();
    for (index, raw_path) in raw_paths.iter().enumerate() {
        let expected_run = format!("run{}", index + 1);
        let observed_run = run_id_from_path(raw_path, "raw trajectory")?;
        if observed_run != expected_run {
            return Err(invalid_input(format!(
                "pooled inputs must be ordered run1, run2, run3; expected {expected_run}, found {observed_run}"
            )).into());
        }
        let raw = load_raw(raw_path)?;
        loaded.push((raw_path.clone(), raw.filename.clone(), raw.rows.len() - HORIZON));
    }
    let expected_total = loaded.iter().map(|(_, _, rows)| *rows).sum::<usize>();

    let pooled_bytes = fs::read(prediction_path)?;
    let mut pooled_reader = ReaderBuilder::new().has_headers(true).flexible(false).from_reader(pooled_bytes.as_slice());
    let headers = pooled_reader.headers()?.clone();
    name_index(&headers, "pooled prediction table")?;
    let mut pooled_row_count = 0usize;
    for record in pooled_reader.records() {
        let _ = record?;
        pooled_row_count += 1;
    }
    if pooled_row_count != expected_total {
        return Err(invalid_input(format!(
            "pooled prediction row count mismatch: raw run windows sum to {expected_total}, pooled table has {pooled_row_count}"
        )).into());
    }

    if output_dir.exists() {
        return Err(invalid_input(format!(
            "pooled output directory '{}' must not already exist; use a fresh path to avoid mixing old and new receipts",
            output_dir.display()
        )).into());
    }
    if let Some(parent) = output_dir.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let output_leaf = output_dir.file_name().and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input("pooled output path must end with a UTF-8 directory name"))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| invalid_input(format!("system clock before UNIX epoch: {error}")))?
        .as_nanos();
    let staging_dir = output_dir.with_file_name(format!(".{output_leaf}.nanodrone-stage-{nonce}"));
    fs::create_dir(&staging_dir)?;

    let staged_receipts: Result<Vec<Value>, Box<dyn Error>> = (|| {
        let mut receipts = Vec::new();
        let mut row_offset = 0usize;
        for (raw_path, raw_filename, row_count) in loaded {
            let output_path = staging_dir.join(raw_filename);
            let receipt = join_files_internal(
                &raw_path, prediction_path, &output_path, true, row_offset, true,
            )?;
            receipts.push(receipt);
            row_offset += row_count;
        }
        Ok(receipts)
    })();
    let receipts = match staged_receipts {
        Ok(receipts) => receipts,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }
    };
    if let Err(error) = fs::rename(&staging_dir, output_dir) {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(error.into());
    }

    Ok(json!({
        "benchmark_id": "symtropy-nanodrone-pooled-run-split-v0.1",
        "claim_ceiling": "pooled_prediction_row_partition_and_truth_attachment_only_no_model_or_safety_claim",
        "source_prediction_file": prediction_path.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
        "source_prediction_sha256": sha256(&pooled_bytes),
        "source_prediction_rows": pooled_row_count,
        "output_directory": output_dir,
        "run_order": ["run1", "run2", "run3"],
        "split_policy": "split pooled rows sequentially using each raw run's exact N-50 window count; each slice is then independently row-alignment checked against raw row i+1",
        "batch_outputs_committed_atomically": true,
        "joined_run_receipts": receipts
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let receipt = if args.len() == 4 && args[3] == "--state-units-si" {
        join_files(
            Path::new(&args[0]),
            Path::new(&args[1]),
            Path::new(&args[2]),
            true,
        )?
    } else if args.len() == 7 && args[5] == "--pooled" && args[6] == "--state-units-si" {
        let raw_paths = vec![
            PathBuf::from(&args[1]),
            PathBuf::from(&args[2]),
            PathBuf::from(&args[3]),
        ];
        join_pooled_predictions(
            Path::new(&args[0]),
            &raw_paths,
            Path::new(&args[4]),
            true,
        )?
    } else {
        return Err(invalid_input(
            "usage (single run): nanodrone_truth_window_join <raw_melon_runN.csv> <predicted_melon_runN.csv> <output_melon_runN.csv> --state-units-si\nusage (pooled upstream test output): nanodrone_truth_window_join <melon_multistep.csv> <raw_melon_run1.csv> <raw_melon_run2.csv> <raw_melon_run3.csv> <output_dir> --pooled --state-units-si",
        )
        .into());
    };
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_raw(path: &Path, rows: usize) {
        let headers = RAW_REQUIRED;
        let mut writer = csv::Writer::from_path(path).unwrap();
        writer.write_record(headers).unwrap();
        for i in 0..rows {
            let t = i as f64 * EXPECTED_DT_S;
            let x = i as f64;
            writer.write_record([
                format!("{t:.8}"),
                format!("{x:.8}"), "0".into(), "0".into(),
                "0".into(), "0".into(), "0".into(),
                "0".into(), "0".into(), "0".into(), "1".into(),
                "0".into(), "0".into(), "0".into(),
                format!("{}", 100 + i), format!("{}", 200 + i),
                format!("{}", 300 + i), format!("{}", 400 + i),
            ]).unwrap();
        }
        writer.flush().unwrap();
    }

    fn write_predictions(path: &Path, rows: usize, base_x_offset: f64) {
        let mut headers = vec![
            "t".to_owned(), "x".into(), "y".into(), "z".into(),
            "vx".into(), "vy".into(), "vz".into(), "wx".into(), "wy".into(), "wz".into(),
            "qx".into(), "qy".into(), "qz".into(), "qw".into(),
        ];
        let fields = [
            "x", "y", "z", "vx", "vy", "vz", "wx", "wy", "wz", "qx", "qy", "qz", "qw",
        ];
        for h in 1..=HORIZON {
            headers.extend(fields.iter().map(|field| prediction_key(field, h)));
        }
        let mut writer = csv::Writer::from_path(path).unwrap();
        writer.write_record(&headers).unwrap();
        for i in 0..rows {
            let mut row = vec![
                format!("{:.8}", i as f64 * EXPECTED_DT_S),
                format!("{}", i as f64 + base_x_offset), "0".into(), "0".into(),
                "0".into(), "0".into(), "0".into(), "0".into(), "0".into(), "0".into(),
                "0".into(), "0".into(), "0".into(), "1".into(),
            ];
            for _ in 1..=HORIZON {
                for field in fields {
                    row.push(if field == "qw" { "1".into() } else { "0".into() });
                }
            }
            writer.write_record(&row).unwrap();
        }
        writer.flush().unwrap();
    }

    #[test]
    fn quaternion_axis_angle_round_trip_preserves_rotation() {
        let angle = 1.2_f64;
        let rotation = [angle / 3.0, angle * 2.0 / 3.0, -angle / 3.0];
        let norm = (rotation.iter().map(|value| value * value).sum::<f64>()).sqrt();
        let scale = (norm * 0.5).sin() / norm;
        let q = [rotation[0] * scale, rotation[1] * scale, rotation[2] * scale, (norm * 0.5).cos()];
        let reconstructed = rotvec_to_quaternion(quaternion_to_rotvec(q));
        assert!(quaternion_distance_sign_invariant(q, reconstructed) < 1.0e-12);
    }

    #[test]
    fn attaches_exact_horizon_truth_without_crossing_run_boundaries() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("symtropy-nanodrone-join-{nonce}"));
        let raw_dir = root.join("data").join("test");
        let prediction_dir = root.join("predictions");
        let output_dir = root.join("joined");
        fs::create_dir_all(&raw_dir).unwrap();
        fs::create_dir_all(&prediction_dir).unwrap();
        // The pooled writer commits a fresh output directory atomically.

        let raw_path = raw_dir.join("melon_20251017_run1.csv");
        let prediction_path = prediction_dir.join("melon_20251017_run1.csv");
        let output_path = output_dir.join("melon_20251017_run1.csv");
        write_raw(&raw_path, 100);
        write_predictions(&prediction_path, 50, 1.0);

        let receipt = join_files(&raw_path, &prediction_path, &output_path, true).unwrap();
        assert_eq!(receipt["design"]["source_row_count"], 100);
        assert_eq!(receipt["design"]["prediction_window_count"], 50);
        assert_eq!(receipt["design"]["target_columns_written"], 13 * HORIZON);
        assert_eq!(receipt["design"]["motor_input_columns_present"], true);

        let mut joined = ReaderBuilder::new().has_headers(true).from_path(&output_path).unwrap();
        let headers = joined.headers().unwrap().clone();
        let index_m1 = headers.iter().position(|header| header == "m1_rads").unwrap();
        let index_m1_t0 = headers.iter().position(|header| header == "m1_rads_input_t0").unwrap();
        let index_m1_t49 = headers.iter().position(|header| header == "m1_rads_input_t49").unwrap();
        let index_x0_x = headers.iter().position(|header| header == "x0_x").unwrap();
        let index_x0_t = headers.iter().position(|header| header == "x0_t").unwrap();
        let index_input_t49 = headers.iter().position(|header| header == "input_time_t49").unwrap();
        let index_x_h1 = headers.iter().position(|header| header == "x_true_h1").unwrap();
        let index_x_h50 = headers.iter().position(|header| header == "x_true_h50").unwrap();
        let rows = joined.records().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows.len(), 50);
        assert_eq!(rows[0].get(index_m1).unwrap().parse::<f64>().unwrap(), 100.0);
        assert_eq!(rows[49].get(index_m1).unwrap().parse::<f64>().unwrap(), 149.0);
        assert_eq!(rows[0].get(index_m1_t0).unwrap().parse::<f64>().unwrap(), 100.0);
        assert_eq!(rows[0].get(index_m1_t49).unwrap().parse::<f64>().unwrap(), 149.0);
        assert_eq!(rows[0].get(index_x0_x).unwrap().parse::<f64>().unwrap(), 0.0);
        assert_eq!(rows[49].get(index_x0_x).unwrap().parse::<f64>().unwrap(), 49.0);
        assert_eq!(rows[0].get(index_x0_t).unwrap().parse::<f64>().unwrap(), 0.0);
        assert_eq!(rows[49].get(index_x0_t).unwrap().parse::<f64>().unwrap(), 0.49);
        assert_eq!(rows[0].get(index_input_t49).unwrap().parse::<f64>().unwrap(), 0.49);
        assert_eq!(rows[0].get(index_x_h1).unwrap().parse::<f64>().unwrap(), 1.0);
        assert_eq!(rows[0].get(index_x_h50).unwrap().parse::<f64>().unwrap(), 50.0);
        assert_eq!(rows[49].get(index_x_h50).unwrap().parse::<f64>().unwrap(), 99.0);

        fs::remove_dir_all(&root).unwrap();
    }

    fn write_pooled_predictions(path: &Path, window_counts: &[usize]) {
        let mut headers = vec![
            "t".to_owned(), "x".into(), "y".into(), "z".into(),
            "vx".into(), "vy".into(), "vz".into(), "wx".into(), "wy".into(), "wz".into(),
            "qx".into(), "qy".into(), "qz".into(), "qw".into(),
        ];
        let fields = [
            "x", "y", "z", "vx", "vy", "vz", "wx", "wy", "wz", "qx", "qy", "qz", "qw",
        ];
        for h in 1..=HORIZON {
            headers.extend(fields.iter().map(|field| prediction_key(field, h)));
        }
        let mut writer = csv::Writer::from_path(path).unwrap();
        writer.write_record(&headers).unwrap();
        for &run_rows in window_counts {
            for i in 0..run_rows {
                let mut row = vec![
                    format!("{:.8}", i as f64 * EXPECTED_DT_S),
                    format!("{}", i + 1), "0".into(), "0".into(),
                    "0".into(), "0".into(), "0".into(), "0".into(), "0".into(), "0".into(),
                    "0".into(), "0".into(), "0".into(), "1".into(),
                ];
                for _ in 1..=HORIZON {
                    for field in fields {
                        row.push(if field == "qw" { "1".into() } else { "0".into() });
                    }
                }
                writer.write_record(&row).unwrap();
            }
        }
        writer.flush().unwrap();
    }

    #[test]
    fn splits_existing_pooled_benchmark_export_by_each_raw_n_minus_horizon() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("symtropy-nanodrone-pooled-{nonce}"));
        let raw_dir = root.join("data").join("test");
        let output_dir = root.join("joined");
        fs::create_dir_all(&raw_dir).unwrap();

        let raw_paths = vec![
            raw_dir.join("melon_20251017_run1.csv"),
            raw_dir.join("melon_20251017_run2.csv"),
            raw_dir.join("melon_20251017_run3.csv"),
        ];
        let raw_lengths = [100usize, 102, 105];
        let window_counts = raw_lengths.iter().map(|length| length - HORIZON).collect::<Vec<_>>();
        for (path, rows) in raw_paths.iter().zip(raw_lengths) {
            write_raw(path, rows);
        }
        let pooled_path = root.join("melon_multistep.csv");
        write_pooled_predictions(&pooled_path, &window_counts);

        let receipt = join_pooled_predictions(&pooled_path, &raw_paths, &output_dir, true).unwrap();
        assert_eq!(receipt["source_prediction_rows"], 157);
        assert_eq!(receipt["joined_run_receipts"].as_array().unwrap().len(), 3);

        for (run_index, expected_rows) in window_counts.iter().enumerate() {
            let output_path = output_dir.join(format!("melon_20251017_run{}.csv", run_index + 1));
            let mut joined = ReaderBuilder::new().has_headers(true).from_path(&output_path).unwrap();
            let headers = joined.headers().unwrap().clone();
            let x_h50 = headers.iter().position(|header| header == "x_true_h50").unwrap();
            let rows = joined.records().collect::<Result<Vec<_>, _>>().unwrap();
            assert_eq!(rows.len(), *expected_rows);
            assert_eq!(rows[0].get(x_h50).unwrap().parse::<f64>().unwrap(), 50.0);
        }

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn refuses_prediction_rows_that_do_not_match_raw_one_step_truth() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("symtropy-nanodrone-join-misaligned-{nonce}"));
        let raw_dir = root.join("data").join("test");
        let prediction_dir = root.join("predictions");
        fs::create_dir_all(&raw_dir).unwrap();
        fs::create_dir_all(&prediction_dir).unwrap();
        let raw_path = raw_dir.join("melon_20251017_run1.csv");
        let prediction_path = prediction_dir.join("melon_20251017_run1.csv");
        let output_path = root.join("joined").join("melon_20251017_run1.csv");
        write_raw(&raw_path, 100);
        write_predictions(&prediction_path, 50, 0.0);
        assert!(join_files(&raw_path, &prediction_path, &output_path, true).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn refuses_run_mismatch_and_missing_si_acknowledgement() {
        assert!(join_files(
            Path::new("/dataset/data/test/melon_20251017_run1.csv"),
            Path::new("/results/melon_20251017_run2.csv"),
            Path::new("/results/joined.csv"),
            true
        ).is_err());
        assert!(join_files(
            Path::new("/dataset/data/test/melon_20251017_run1.csv"),
            Path::new("/results/melon_20251017_run1.csv"),
            Path::new("/results/joined.csv"),
            false
        ).is_err());
    }
}
