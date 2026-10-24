//! Fail-closed integrity validator for the public IDSIA nano-drone CSV trajectories.
//!
//! This checks raw input integrity before any model code can consume the data. It does
//! not train a model, reproduce the upstream metric, or establish a real-flight win.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;
use serde_json::json;
use sha2::{Digest, Sha256};

const SCHEMA_ID: &str = "nanodrone-csv-trajectory-validation-v0.1";
const EXPECTED_DT_S: f64 = 0.010;
const MAX_DT_DEVIATION_S: f64 = 0.002;
const QUATERNION_NORM_MIN: f64 = 0.95;
const QUATERNION_NORM_MAX: f64 = 1.05;

// Order matches the public loader's XYZW quaternion convention and the four motor-speed
// channels. Header order in the CSV itself is not assumed.
const REQUIRED_COLUMNS: [&str; 18] = [
    "t", "x", "y", "z", "vx", "vy", "vz", "qx", "qy", "qz", "qw", "wx", "wy", "wz",
    "m1_rads", "m2_rads", "m3_rads", "m4_rads",
];
const MOTOR_COLUMNS: [&str; 4] = ["m1_rads", "m2_rads", "m3_rads", "m4_rads"];

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn validate_split_policy(split: &str, trajectory: &str, run_id: &str) -> Result<(), io::Error> {
    const TRAIN_TRAJECTORIES: [&str; 3] = ["square", "random", "chirp"];
    const TRAIN_RUNS: [&str; 3] = ["run1", "run2", "run3"];
    const TEST_RUNS: [&str; 3] = ["run1", "run2", "run3"];
    const ALL_TRAJECTORIES: [&str; 4] = ["square", "random", "chirp", "melon"];

    if !ALL_TRAJECTORIES.contains(&trajectory) {
        return Err(invalid_input(format!(
            "unsupported trajectory '{trajectory}'; expected square, random, chirp, or melon"
        )));
    }
    if !["run1", "run2", "run3", "run4"].contains(&run_id) {
        return Err(invalid_input(
            "run-id must be one of run1, run2, run3, or run4 for the pinned benchmark protocol",
        ));
    }

    match split {
        "train" if TRAIN_TRAJECTORIES.contains(&trajectory) && TRAIN_RUNS.contains(&run_id) => Ok(()),
        "validation" if TRAIN_TRAJECTORIES.contains(&trajectory) && run_id == "run4" => Ok(()),
        "test" if trajectory == "melon" && TEST_RUNS.contains(&run_id) => Ok(()),
        "train" => Err(invalid_input(
            "training requires Square/Random/Chirp run1-run3; Melon and run4 are not training data",
        )),
        "validation" => Err(invalid_input(
            "validation requires Square/Random/Chirp run4 from the training directory",
        )),
        "test" => Err(invalid_input(
            "held-out evaluation requires Melon run1-run3; training trajectories and run4 are not test data",
        )),
        _ => Err(invalid_input("split must be exactly 'train', 'validation', or 'test'")),
    }
}

fn validate_filename_and_directory(
    path: &Path,
    filename: &str,
    split: &str,
    trajectory: &str,
    run_id: &str,
) -> Result<(), io::Error> {
    let expected_directory = if split == "test" { "test" } else { "train" };
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if parent != expected_directory {
        return Err(invalid_input(format!(
            "split '{split}' expects the official data/{expected_directory}/ directory, found parent '{parent}'"
        )));
    }

    let stem = path.file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input("CSV path must have a UTF-8 file stem"))?
        .to_ascii_lowercase();
    let parts = stem.split('_').collect::<Vec<_>>();
    if parts.first().copied() != Some(trajectory) || parts.last().copied() != Some(run_id) {
        return Err(invalid_input(format!(
            "file name '{filename}' must start with trajectory '{trajectory}' and end with exact run token '{run_id}'"
        )));
    }
    Ok(())
}

fn validate_dt(dt: f64) -> Result<(), io::Error> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err(invalid_input(format!(
            "timestamps must be strictly increasing; observed dt={dt}"
        )));
    }
    let deviation = (dt - EXPECTED_DT_S).abs();
    if deviation > MAX_DT_DEVIATION_S {
        return Err(invalid_input(format!(
            "sample interval {dt:.9}s differs from the declared 100 Hz period by {deviation:.9}s (limit {MAX_DT_DEVIATION_S:.9}s)"
        )));
    }
    Ok(())
}

fn parse_required_row(
    record: &csv::StringRecord,
    indices: &HashMap<String, usize>,
    row_number: usize,
) -> Result<[f64; 18], Box<dyn Error>> {
    let mut values = [0.0_f64; 18];
    for (column_index, column_name) in REQUIRED_COLUMNS.iter().enumerate() {
        let index = *indices
            .get(*column_name)
            .ok_or_else(|| invalid_input(format!("missing required column '{column_name}'")))?;
        let raw = record.get(index).ok_or_else(|| {
            invalid_input(format!("row {row_number}: missing value for '{column_name}'"))
        })?;
        let value = raw.parse::<f64>().map_err(|error| {
            invalid_input(format!(
                "row {row_number}: '{column_name}' is not a numeric value ({error})"
            ))
        })?;
        if !value.is_finite() {
            return Err(invalid_input(format!(
                "row {row_number}: '{column_name}' is non-finite"
            ))
            .into());
        }
        values[column_index] = value;
    }
    Ok(values)
}

fn validate_file(
    path: &Path,
    split: &str,
    trajectory: &str,
    run_id: &str,
) -> Result<serde_json::Value, Box<dyn Error>> {
    validate_split_policy(split, trajectory, run_id)?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input("CSV path must have a UTF-8 file name"))?;
    validate_filename_and_directory(path, filename, split, trajectory, run_id)?;
    if filename.rsplit_once('.').map(|(_, extension)| extension) != Some("csv") {
        return Err(invalid_input("input must be a .csv file using the official public loader's tabular format").into());
    }

    let bytes = fs::read(path)?;
    if bytes.is_empty() {
        return Err(invalid_input("CSV file is empty").into());
    }
    let file_sha256 = format!("{:x}", Sha256::digest(&bytes));

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(bytes.as_slice());
    let headers = reader.headers()?.clone();
    let mut indices: HashMap<String, usize> = HashMap::new();
    for (index, header) in headers.iter().enumerate() {
        if header.trim().is_empty() {
            return Err(invalid_input(format!("header column {index} has an empty name")).into());
        }
        if indices.insert(header.to_owned(), index).is_some() {
            return Err(invalid_input(format!("duplicate CSV header '{header}'")).into());
        }
    }
    for required in REQUIRED_COLUMNS {
        if !indices.contains_key(required) {
            return Err(invalid_input(format!(
                "missing required official-schema column '{required}'"
            ))
            .into());
        }
    }

    let mut rows = 0usize;
    let mut previous_t: Option<f64> = None;
    let mut intervals = Vec::new();
    let mut min_quaternion_norm = f64::INFINITY;
    let mut max_quaternion_norm = 0.0_f64;
    let mut minimum_motor_speed = f64::INFINITY;
    let mut maximum_motor_speed = 0.0_f64;

    for result in reader.records() {
        let record = result?;
        rows += 1;
        let values = parse_required_row(&record, &indices, rows + 1)?;
        let timestamp = values[0];

        if let Some(prior) = previous_t {
            let dt = timestamp - prior;
            validate_dt(dt).map_err(|error| {
                invalid_input(format!("row {}: {error}", rows + 1))
            })?;
            intervals.push(dt);
        }
        previous_t = Some(timestamp);

        let qx = values[7];
        let qy = values[8];
        let qz = values[9];
        let qw = values[10];
        let quaternion_norm = (qx * qx + qy * qy + qz * qz + qw * qw).sqrt();
        if !quaternion_norm.is_finite()
            || !(QUATERNION_NORM_MIN..=QUATERNION_NORM_MAX).contains(&quaternion_norm)
        {
            return Err(invalid_input(format!(
                "row {}: quaternion norm {quaternion_norm} outside [{QUATERNION_NORM_MIN}, {QUATERNION_NORM_MAX}]",
                rows + 1
            ))
            .into());
        }
        min_quaternion_norm = min_quaternion_norm.min(quaternion_norm);
        max_quaternion_norm = max_quaternion_norm.max(quaternion_norm);

        for column in MOTOR_COLUMNS {
            let value_index = REQUIRED_COLUMNS
                .iter()
                .position(|required| *required == column)
                .expect("required motor column checked");
            let value = values[value_index];
            if value < 0.0 {
                return Err(invalid_input(format!(
                    "row {}: motor angular velocity '{column}' must be non-negative",
                    rows + 1
                ))
                .into());
            }
            minimum_motor_speed = minimum_motor_speed.min(value);
            maximum_motor_speed = maximum_motor_speed.max(value);
        }
    }

    if rows < 2 {
        return Err(invalid_input(format!(
            "trajectory must have at least 2 rows to validate intervals; found {rows}"
        ))
        .into());
    }
    let mut sorted_intervals = intervals.clone();
    sorted_intervals.sort_by(f64::total_cmp);
    let median_dt = if sorted_intervals.len() % 2 == 0 {
        let n = sorted_intervals.len();
        (sorted_intervals[n / 2 - 1] + sorted_intervals[n / 2]) / 2.0
    } else {
        sorted_intervals[sorted_intervals.len() / 2]
    };
    let maximum_dt_deviation = intervals
        .iter()
        .map(|dt| (dt - EXPECTED_DT_S).abs())
        .fold(0.0_f64, f64::max);

    Ok(json!({
        "schema_id": SCHEMA_ID,
        "claim_ceiling": "raw_dataset_integrity_and_split_gate_only_no_model_or_real_flight_performance_claim",
        "source": {
            "repository": "https://github.com/idsia-robotics/nanodrone-sysid-benchmark",
            "upstream_schema_reference": "dataset/dataset.py::QuadDataset"
        },
        "file": {
            "name": filename,
            "sha256": file_sha256,
            "trajectory": trajectory,
            "run_id": run_id,
            "split": split
        },
        "integrity": {
            "valid": true,
            "row_count": rows,
            "required_columns": REQUIRED_COLUMNS,
            "extra_columns_ignored_but_preserved_in_source_hash": true,
            "timestamp_column": "t (seconds)",
            "expected_sample_period_s": EXPECTED_DT_S,
            "maximum_allowed_sample_period_deviation_s": MAX_DT_DEVIATION_S,
            "median_sample_period_s": median_dt,
            "maximum_observed_sample_period_deviation_s": maximum_dt_deviation,
            "quaternion_columns_and_order": ["qx", "qy", "qz", "qw"],
            "quaternion_norm_range_observed": [min_quaternion_norm, max_quaternion_norm],
            "quaternion_norm_acceptance_range": [QUATERNION_NORM_MIN, QUATERNION_NORM_MAX],
            "motor_speed_columns": MOTOR_COLUMNS,
            "motor_speed_unit": "radians_per_second",
            "motor_speed_range_observed": [minimum_motor_speed, maximum_motor_speed]
        },
        "state_convention": {
            "raw_state_dimension": 13,
            "raw_state_order": ["x", "y", "z", "vx", "vy", "vz", "qx", "qy", "qz", "qw", "wx", "wy", "wz"],
            "model_state_order": ["x", "y", "z", "vx", "vy", "vz", "rotvec_x", "rotvec_y", "rotvec_z", "wx", "wy", "wz"],
            "model_state_dimension": 12,
            "model_orientation_representation": "SO(3) axis-angle rotation vector per upstream quaternion_to_axis_angle",
            "angular_velocity_frame": "body",
            "position_and_linear_velocity_frame": "world",
            "quaternion_normalization_or_sign_correction_applied": false
        },
        "split_policy": {
            "training_trajectories": ["square", "random", "chirp"],
            "held_out_test_trajectory": "melon",
            "training_leakage_checked": true,
            "trajectory_id_must_not_be_split_across_train_and_test": true
        },
        "next_required_gate": "reproduce_an_upstream_reference_model_and_official_multi_horizon_metrics_before_reporting_Symthaea_scores"
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(invalid_input(
            "usage: nanodrone_dataset_validate <CSV_PATH> <train|validation|test> <square|random|chirp|melon> <run1|run2|run3|run4>",
        )
        .into());
    }

    let path = PathBuf::from(&args[1]);
    let receipt = validate_file(&path, &args[2], &args[3], &args[4])?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_trajectory_and_run_split_is_enforced() {
        for trajectory in ["square", "random", "chirp"] {
            for run_id in ["run1", "run2", "run3"] {
                assert!(validate_split_policy("train", trajectory, run_id).is_ok());
            }
            assert!(validate_split_policy("validation", trajectory, "run4").is_ok());
            assert!(validate_split_policy("train", trajectory, "run4").is_err());
            assert!(validate_split_policy("validation", trajectory, "run1").is_err());
            assert!(validate_split_policy("test", trajectory, "run1").is_err());
        }
        for run_id in ["run1", "run2", "run3"] {
            assert!(validate_split_policy("test", "melon", run_id).is_ok());
        }
        assert!(validate_split_policy("train", "melon", "run1").is_err());
        assert!(validate_split_policy("validation", "melon", "run4").is_err());
        assert!(validate_split_policy("test", "melon", "run4").is_err());
    }

    #[test]
    fn filenames_require_exact_trajectory_and_run_tokens() {
        assert!(validate_filename_and_directory(
            Path::new("/dataset/data/train/square_20251017_run1.csv"),
            "square_20251017_run1.csv", "train", "square", "run1"
        ).is_ok());
        assert!(validate_filename_and_directory(
            Path::new("/dataset/data/test/melon_20251017_run1.csv"),
            "melon_20251017_run1.csv", "test", "melon", "run1"
        ).is_ok());
        assert!(validate_filename_and_directory(
            Path::new("/dataset/data/test/melon_20251017_run10.csv"),
            "melon_20251017_run10.csv", "test", "melon", "run1"
        ).is_err());
        assert!(validate_filename_and_directory(
            Path::new("/dataset/data/test/melon_20251017_run1.csv"),
            "melon_20251017_run1.csv", "train", "melon", "run1"
        ).is_err());
    }

    #[test]
    fn sample_interval_gate_rejects_duplicate_gapped_and_jittered_rows() {
        assert!(validate_dt(0.010).is_ok());
        assert!(validate_dt(0.0115).is_ok());
        assert!(validate_dt(0.0).is_err());
        assert!(validate_dt(-0.01).is_err());
        assert!(validate_dt(0.020).is_err());
        assert!(validate_dt(f64::NAN).is_err());
    }

    #[test]
    fn run_id_cannot_inject_path_components() {
        assert!(validate_split_policy("train", "square", "run1").is_ok());
        assert!(validate_split_policy("train", "square", "../melon").is_err());
    }

    #[test]
    fn required_columns_are_read_by_name_not_by_csv_order() {
        let headers = REQUIRED_COLUMNS.iter().rev().map(|name| (*name).to_owned()).collect::<Vec<_>>();
        let indices = headers.iter().enumerate()
            .map(|(index, name)| (name.clone(), index))
            .collect::<HashMap<_, _>>();
        let row = headers.iter().map(|name| {
            let required_index = REQUIRED_COLUMNS.iter().position(|required| *required == name.as_str()).unwrap();
            (required_index as f64).to_string()
        }).collect::<Vec<_>>();
        let record = csv::StringRecord::from(row);
        let parsed = parse_required_row(&record, &indices, 2).unwrap();
        for (index, value) in parsed.iter().enumerate() {
            assert_eq!(*value, index as f64);
        }
    }
}
