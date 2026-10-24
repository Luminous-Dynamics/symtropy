//! Run-isolated multi-horizon metrics for Nano-Quadrotor prediction CSVs.
//!
//! Mirrors the public benchmark's vector-norm errors and SO(3) geodesic orientation
//! metric, while preserving independent run boundaries. The source-compatible shifted
//! truth metric is reported separately from an all-available-window metric, which
//! requires explicit per-horizon truth columns. Never concatenate runs before scoring.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use csv::ReaderBuilder;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const HORIZON: usize = 50;
const EXPECTED_DT_S: f64 = 0.010;
const MAX_DT_DEVIATION_S: f64 = 0.002;
const QUATERNION_NORM_MIN: f64 = 0.95;
const QUATERNION_NORM_MAX: f64 = 1.05;

const COMMON_FIELDS: [&str; 9] = [
    "x", "y", "z", "vx", "vy", "vz", "wx", "wy", "wz",
];
const QUATERNION_FIELDS: [&str; 4] = ["qx", "qy", "qz", "qw"];
const ROTVEC_FIELDS: [&str; 3] = ["rx", "ry", "rz"];
const MOTOR_INPUT_FIELDS: [&str; 4] = ["m1_rads", "m2_rads", "m3_rads", "m4_rads"];
const POSITION_FIELDS: [&str; 3] = ["x", "y", "z"];
const VELOCITY_FIELDS: [&str; 3] = ["vx", "vy", "vz"];
const ANGULAR_RATE_FIELDS: [&str; 3] = ["wx", "wy", "wz"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone)]
struct RunInput {
    filename: String,
    run_id: String,
    sha256: String,
    ground_truth_window_sha256: Option<String>,
    conditioning_window_sha256: Option<String>,
    orientation_mode: OrientationMode,
    rows: Vec<HashMap<String, f64>>,
    has_complete_window_truth: bool,
}

#[derive(Debug, Clone, Copy)]
struct Metrics {
    position_m: f64,
    velocity_m_s: f64,
    orientation_rad: f64,
    orientation_sign_invariant_rad: f64,
    angular_rate_rad_s: f64,
}

impl Metrics {
    fn zero() -> Self {
        Self {
            position_m: 0.0,
            velocity_m_s: 0.0,
            orientation_rad: 0.0,
            orientation_sign_invariant_rad: 0.0,
            angular_rate_rad_s: 0.0,
        }
    }

    fn add_assign(&mut self, other: Self) {
        self.position_m += other.position_m;
        self.velocity_m_s += other.velocity_m_s;
        self.orientation_rad += other.orientation_rad;
        self.orientation_sign_invariant_rad += other.orientation_sign_invariant_rad;
        self.angular_rate_rad_s += other.angular_rate_rad_s;
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            position_m: self.position_m * factor,
            velocity_m_s: self.velocity_m_s * factor,
            orientation_rad: self.orientation_rad * factor,
            orientation_sign_invariant_rad: self.orientation_sign_invariant_rad * factor,
            angular_rate_rad_s: self.angular_rate_rad_s * factor,
        }
    }

    fn to_json(self) -> Value {
        json!({
            "position_mean_euclidean_error_m": self.position_m,
            "linear_velocity_mean_euclidean_error_m_s": self.velocity_m_s,
            "orientation_geodesic_error_rad_reference_formula": self.orientation_rad,
            "orientation_geodesic_error_rad_sign_invariant": self.orientation_sign_invariant_rad,
            "body_angular_rate_mean_euclidean_error_rad_s": self.angular_rate_rad_s
        })
    }
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn state_fields(mode: OrientationMode) -> Vec<&'static str> {
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

fn conditioning_column_names(mode: OrientationMode) -> Vec<String> {
    let mut columns = vec!["x0_t".to_owned()];
    columns.extend(state_fields(mode).iter().map(|field| input_state_key(field)));
    columns.extend((0..HORIZON).map(motor_sequence_time_key));
    columns.extend(MOTOR_INPUT_FIELDS.iter().flat_map(|field|
        (0..HORIZON).map(move |step| motor_sequence_key(field, step))
    ));
    columns
}

fn required_numeric_columns(mode: OrientationMode, complete_targets: bool) -> HashSet<String> {
    let mut columns = HashSet::new();
    columns.insert("t".to_owned());
    for field in state_fields(mode) {
        columns.insert(field.to_owned());
        for horizon in 1..=HORIZON {
            columns.insert(prediction_key(field, horizon));
            if complete_targets {
                columns.insert(target_key(field, horizon));
            }
        }
    }
    columns
}

fn check_quaternion_norm(q: [f64; 4], label: &str) -> Result<(), io::Error> {
    let norm = q.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || !(QUATERNION_NORM_MIN..=QUATERNION_NORM_MAX).contains(&norm) {
        return Err(invalid_input(format!(
            "{label}: quaternion norm {norm} outside [{QUATERNION_NORM_MIN}, {QUATERNION_NORM_MAX}]"
        )));
    }
    Ok(())
}

fn validate_conditioning_timestamps(row: &HashMap<String, f64>, label: &str) -> Result<(), io::Error> {
    let x0_time = row["x0_t"];
    let first_input_time = row[&motor_sequence_time_key(0)];
    if (x0_time - first_input_time).abs() > 1.0e-9 {
        return Err(invalid_input(format!(
            "{label}: x0_t={x0_time} does not match first motor input timestamp {first_input_time}"
        )));
    }
    let mut previous = first_input_time;
    for step in 1..HORIZON {
        let current = row[&motor_sequence_time_key(step)];
        let dt = current - previous;
        if !dt.is_finite() || dt <= 0.0 || (dt - EXPECTED_DT_S).abs() > MAX_DT_DEVIATION_S {
            return Err(invalid_input(format!(
                "{label}: motor input timestamp step {step} has dt={dt:.9}s outside 100 Hz tolerance"
            )));
        }
        previous = current;
    }
    Ok(())
}

fn rotvec_to_quaternion(r: [f64; 3]) -> [f64; 4] {
    let angle = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    if angle <= f64::EPSILON {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let scale = (angle * 0.5).sin() / angle;
    [r[0] * scale, r[1] * scale, r[2] * scale, (angle * 0.5).cos()]
}

fn quaternion_conjugate(q: [f64; 4]) -> [f64; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

/// Hamilton product using the public benchmark's XYZW convention.
fn quaternion_multiply(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let [x1, y1, z1, w1] = a;
    let [x2, y2, z2, w2] = b;
    [
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
    ]
}

/// Exact published-source formula: 2 atan2(||vector(q_true^-1 * q_pred)||, w).
/// It intentionally does not canonicalize quaternion sign, to make the implementation
/// comparable to upstream; a sign-invariant metric can be added as a separately named
/// diagnostic rather than silently changing the reference score.
fn quaternion_geodesic_error(q_true: [f64; 4], q_pred: [f64; 4]) -> f64 {
    let relative = quaternion_multiply(quaternion_conjugate(q_true), q_pred);
    let vector_norm =
        (relative[0] * relative[0] + relative[1] * relative[1] + relative[2] * relative[2])
            .sqrt();
    2.0 * vector_norm.atan2(relative[3])
}

fn euclidean_error(
    truth: &HashMap<String, f64>,
    prediction: &HashMap<String, f64>,
    fields: &[&str],
    target_horizon: Option<usize>,
    prediction_horizon: usize,
) -> f64 {
    let sum = fields.iter().map(|field| {
        let truth_key = target_horizon
            .map(|horizon| target_key(field, horizon))
            .unwrap_or_else(|| (*field).to_owned());
        let prediction_key = prediction_key(field, prediction_horizon);
        let delta = truth[&truth_key] - prediction[&prediction_key];
        delta * delta
    }).sum::<f64>();
    sum.sqrt()
}

fn orientation_quaternions(
    truth: &HashMap<String, f64>,
    prediction: &HashMap<String, f64>,
    mode: OrientationMode,
    target_horizon: Option<usize>,
    prediction_horizon: usize,
) -> ([f64; 4], [f64; 4]) {
    let true_component = |field: &str| {
        let key = target_horizon
            .map(|horizon| target_key(field, horizon))
            .unwrap_or_else(|| field.to_owned());
        truth[&key]
    };
    let predicted_component = |field: &str| {
        prediction[&prediction_key(field, prediction_horizon)]
    };

    match mode {
        OrientationMode::Quaternion => (
            [
                true_component("qx"),
                true_component("qy"),
                true_component("qz"),
                true_component("qw"),
            ],
            [
                predicted_component("qx"),
                predicted_component("qy"),
                predicted_component("qz"),
                predicted_component("qw"),
            ],
        ),
        OrientationMode::RotationVector => (
            rotvec_to_quaternion([
                true_component("rx"),
                true_component("ry"),
                true_component("rz"),
            ]),
            rotvec_to_quaternion([
                predicted_component("rx"),
                predicted_component("ry"),
                predicted_component("rz"),
            ]),
        ),
    }
}

/// Sign-invariant SO(3) distance using normalized quaternions. Unlike the upstream
/// source formula, this correctly treats q and -q as the same orientation. It is a
/// supplementary diagnostic and never replaces the reference-compatible metric.
fn quaternion_geodesic_error_sign_invariant(q_true: [f64; 4], q_pred: [f64; 4]) -> f64 {
    let norm_true = q_true.iter().map(|value| value * value).sum::<f64>().sqrt();
    let norm_pred = q_pred.iter().map(|value| value * value).sum::<f64>().sqrt();
    let dot = q_true.iter().zip(q_pred.iter()).map(|(a, b)| a * b).sum::<f64>()
        / (norm_true * norm_pred);
    2.0 * dot.abs().clamp(0.0, 1.0).acos()
}

fn metrics_for_pair(
    truth: &HashMap<String, f64>,
    prediction: &HashMap<String, f64>,
    mode: OrientationMode,
    target_horizon: Option<usize>,
    prediction_horizon: usize,
) -> Metrics {
    let (q_true, q_pred) = orientation_quaternions(
        truth, prediction, mode, target_horizon, prediction_horizon,
    );
    Metrics {
        position_m: euclidean_error(
            truth, prediction, &POSITION_FIELDS, target_horizon, prediction_horizon,
        ),
        velocity_m_s: euclidean_error(
            truth, prediction, &VELOCITY_FIELDS, target_horizon, prediction_horizon,
        ),
        orientation_rad: quaternion_geodesic_error(q_true, q_pred),
        orientation_sign_invariant_rad: quaternion_geodesic_error_sign_invariant(q_true, q_pred),
        angular_rate_rad_s: euclidean_error(
            truth, prediction, &ANGULAR_RATE_FIELDS, target_horizon, prediction_horizon,
        ),
    }
}

fn score_run(run: &RunInput, complete_windows: bool) -> Value {
    let n = run.rows.len();
    let mut per_horizon = Vec::with_capacity(HORIZON);
    let mut cumulative = Metrics::zero();

    for horizon in 1..=HORIZON {
        let sample_count = if complete_windows {
            n
        } else {
            n.saturating_sub(horizon - 1)
        };
        assert!(sample_count > 0, "not enough rows for horizon {horizon}");

        let mut summed = Metrics::zero();
        for index in 0..sample_count {
            let truth_index = if complete_windows {
                index
            } else {
                index + horizon - 1
            };
            let truth = &run.rows[truth_index];
            let prediction = &run.rows[index];
            summed.add_assign(metrics_for_pair(
                truth,
                prediction,
                run.orientation_mode,
                if complete_windows { Some(horizon) } else { None },
                horizon,
            ));
        }
        let average = summed.scale(1.0 / sample_count as f64);
        cumulative.add_assign(average);
        per_horizon.push(json!({
            "horizon_steps": horizon,
            "sample_count": sample_count,
            "available_prediction_windows": n,
            "prediction_windows_not_scored_by_shifted_truth_alignment": if complete_windows { 0 } else { horizon - 1 },
            "metrics": average.to_json()
        }));
    }

    json!({
        "alignment": if complete_windows {
            "same-row-per-horizon-truth_columns; all available prediction windows scored"
        } else {
            "match prediction row i to base truth row i+h-1; source-compatible shift(-h+1) formula, scored independently within this run"
        },
        "per_horizon": per_horizon,
        "simulation_error_sum_over_horizons_1_to_50": cumulative.to_json()
    })
}

fn run_id_from_filename(path: &Path) -> Result<(String, String), io::Error> {
    let filename = path.file_name().and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input("prediction path needs a UTF-8 filename"))?;
    if path.extension().and_then(|value| value.to_str()) != Some("csv") {
        return Err(invalid_input(format!("'{filename}' must use the .csv extension")));
    }
    let stem = path.file_stem().and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input("prediction path needs a UTF-8 file stem"))?
        .to_ascii_lowercase();
    let parts = stem.split('_').collect::<Vec<_>>();
    let run_id = parts.last().copied().unwrap_or_default();
    if parts.first().copied() != Some("melon") || !["run1", "run2", "run3"].contains(&run_id) {
        return Err(invalid_input(format!(
            "prediction filename '{filename}' must begin with 'melon' and end with run1, run2, or run3"
        )));
    }
    Ok((filename.to_owned(), run_id.to_owned()))
}

fn load_run(path: &Path) -> Result<RunInput, Box<dyn Error>> {
    let (filename, run_id) = run_id_from_filename(path)?;
    let bytes = fs::read(path)?;
    if bytes.is_empty() {
        return Err(invalid_input(format!("{filename}: file is empty")).into());
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let mut reader = ReaderBuilder::new().has_headers(true).flexible(false).from_reader(bytes.as_slice());
    let headers = reader.headers()?.clone();
    let mut header_indices: HashMap<String, usize> = HashMap::new();
    for (index, header) in headers.iter().enumerate() {
        if header.trim().is_empty() {
            return Err(invalid_input(format!("{filename}: empty header at column {index}")).into());
        }
        if header_indices.insert(header.to_owned(), index).is_some() {
            return Err(invalid_input(format!("{filename}: duplicate header '{header}'")).into());
        }
    }

    let has_complete_quaternions =
        QUATERNION_FIELDS.iter().all(|field| header_indices.contains_key(*field))
        && (1..=HORIZON).all(|h| QUATERNION_FIELDS.iter().all(|field| header_indices.contains_key(&prediction_key(field, h))));
    let has_complete_rotvec =
        ROTVEC_FIELDS.iter().all(|field| header_indices.contains_key(*field))
        && (1..=HORIZON).all(|h| ROTVEC_FIELDS.iter().all(|field| header_indices.contains_key(&prediction_key(field, h))));
    let orientation_mode = if has_complete_quaternions {
        OrientationMode::Quaternion
    } else if has_complete_rotvec {
        OrientationMode::RotationVector
    } else {
        return Err(invalid_input(format!(
            "{filename}: need complete true and 1..={HORIZON} predicted quaternion or rotation-vector columns"
        )).into());
    };

    for field in COMMON_FIELDS {
        if !header_indices.contains_key(field) {
            return Err(invalid_input(format!("{filename}: missing truth column '{field}'")).into());
        }
    }
    if !header_indices.contains_key("t") {
        return Err(invalid_input(format!("{filename}: missing time column 't'")).into());
    }

    let full_target_columns = state_fields(orientation_mode).iter()
        .flat_map(|field| (1..=HORIZON).map(move |h| target_key(field, h)))
        .collect::<Vec<_>>();
    let target_count = full_target_columns.iter().filter(|key| header_indices.contains_key(key.as_str())).count();
    let has_complete_window_truth = if target_count == 0 {
        false
    } else if target_count == full_target_columns.len() {
        true
    } else {
        return Err(invalid_input(format!(
            "{filename}: partial per-horizon truth schema ({target_count}/{} columns); provide all or none",
            full_target_columns.len()
        )).into());
    };

    let motor_input_count = MOTOR_INPUT_FIELDS.iter()
        .filter(|field| header_indices.contains_key(**field))
        .count();
    if motor_input_count != 0 && motor_input_count != MOTOR_INPUT_FIELDS.len() {
        return Err(invalid_input(format!(
            "{filename}: partial base motor-input schema ({motor_input_count}/{} columns); provide all four or none",
            MOTOR_INPUT_FIELDS.len()
        )).into());
    }
    let has_base_motor_inputs = motor_input_count == MOTOR_INPUT_FIELDS.len();

    let conditioning_columns = conditioning_column_names(orientation_mode);
    let conditioning_count = conditioning_columns.iter()
        .filter(|column| header_indices.contains_key(column.as_str()))
        .count();
    if conditioning_count != 0 && conditioning_count != conditioning_columns.len() {
        return Err(invalid_input(format!(
            "{filename}: partial full-conditioning schema ({conditioning_count}/{} x0/input-sequence columns); provide all or none",
            conditioning_columns.len()
        )).into());
    }
    let has_complete_conditioning_signals = conditioning_count == conditioning_columns.len();

    let mut required = required_numeric_columns(orientation_mode, has_complete_window_truth);
    if has_base_motor_inputs {
        required.extend(MOTOR_INPUT_FIELDS.iter().map(|field| (*field).to_owned()));
    }
    if has_complete_conditioning_signals {
        required.extend(conditioning_columns.iter().cloned());
    }
    // In quaternion mode, also require the selected true quaternion and predictions.
    // In rotation-vector mode the metric conversion is mathematical and needs only rotvecs.
    for key in [
        "x", "y", "z", "vx", "vy", "vz", "wx", "wy", "wz",
    ] {
        required.insert(key.to_owned());
    }
    for key in &required {
        if !header_indices.contains_key(key.as_str()) {
            return Err(invalid_input(format!("{filename}: missing required numeric column '{key}'")).into());
        }
    }

    let mut rows = Vec::new();
    let mut previous_time = None::<f64>;
    for (row_index, record_result) in reader.records().enumerate() {
        let record = record_result?;
        let mut row = HashMap::with_capacity(required.len());
        for key in &required {
            let column_index = header_indices[key];
            let raw = record.get(column_index).ok_or_else(|| {
                invalid_input(format!("{filename}: row {} has no value for '{key}'", row_index + 2))
            })?;
            let value = raw.parse::<f64>().map_err(|error| {
                invalid_input(format!("{filename}: row {} column '{key}' is not numeric: {error}", row_index + 2))
            })?;
            if !value.is_finite() {
                return Err(invalid_input(format!("{filename}: row {} column '{key}' is non-finite", row_index + 2)).into());
            }
            row.insert(key.clone(), value);
        }
        let time = row["t"];
        if let Some(previous) = previous_time {
            let dt = time - previous;
            if !dt.is_finite() || dt <= 0.0 || (dt - EXPECTED_DT_S).abs() > MAX_DT_DEVIATION_S {
                return Err(invalid_input(format!(
                    "{filename}: row {} timestamp step {dt:.9}s outside the declared 100 Hz tolerance",
                    row_index + 2
                )).into());
            }
        }
        previous_time = Some(time);

        if orientation_mode == OrientationMode::Quaternion {
            check_quaternion_norm(
                [row["qx"], row["qy"], row["qz"], row["qw"]],
                &format!("{filename} row {} truth", row_index + 2),
            )?;
            if has_complete_conditioning_signals {
                check_quaternion_norm(
                    [
                        row["x0_qx"], row["x0_qy"], row["x0_qz"], row["x0_qw"],
                    ],
                    &format!("{filename} row {} initial-state orientation", row_index + 2),
                )?;
            }
            for horizon in 1..=HORIZON {
                let q = [
                    row[&prediction_key("qx", horizon)],
                    row[&prediction_key("qy", horizon)],
                    row[&prediction_key("qz", horizon)],
                    row[&prediction_key("qw", horizon)],
                ];
                check_quaternion_norm(q, &format!("{filename} row {} prediction h={horizon}", row_index + 2))?;
            }
            if has_complete_window_truth {
                for horizon in 1..=HORIZON {
                    let q = [
                        row[&target_key("qx", horizon)],
                        row[&target_key("qy", horizon)],
                        row[&target_key("qz", horizon)],
                        row[&target_key("qw", horizon)],
                    ];
                    check_quaternion_norm(q, &format!("{filename} row {} target h={horizon}", row_index + 2))?;
                }
            }
        }
        if has_complete_conditioning_signals {
            validate_conditioning_timestamps(&row, &format!("{filename} row {}", row_index + 2))?;
            for field in MOTOR_INPUT_FIELDS {
                for input_step in 0..HORIZON {
                    let motor = row[&motor_sequence_key(field, input_step)];
                    if motor < 0.0 {
                        return Err(invalid_input(format!(
                            "{filename} row {}: motor input {field} at step {input_step} must be non-negative",
                            row_index + 2
                        )).into());
                    }
                }
            }
        }
        rows.push(row);
    }

    if rows.len() < HORIZON {
        return Err(invalid_input(format!("{filename}: expected at least {HORIZON} prediction rows; found {}", rows.len())).into());
    }

    let ground_truth_window_sha256 = if has_complete_window_truth {
        // Hash canonical numeric values rather than CSV spellings so "1" and "1.0"
        // match, while any changed timestamp or target value invalidates the pair.
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy-nanodrone-ground-truth-windows-v1\0");
        hasher.update((rows.len() as u64).to_le_bytes());
        for row in &rows {
            hasher.update(row["t"].to_bits().to_le_bytes());
            for horizon in 1..=HORIZON {
                for field in state_fields(orientation_mode) {
                    hasher.update(row[&target_key(field, horizon)].to_bits().to_le_bytes());
                }
            }
        }
        Some(format!("{:x}", hasher.finalize()))
    } else {
        None
    };

    let conditioning_window_sha256 = if has_complete_conditioning_signals {
        // Match QuadDataset exactly: each horizon prediction is conditioned on x0
        // at raw row i and the entire u_seq of 50 four-motor samples i..i+49.
        // The old one-step motor columns alone are not enough to certify pairing.
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy-nanodrone-conditioning-windows-v2\0");
        hasher.update((rows.len() as u64).to_le_bytes());
        for row in &rows {
            hasher.update(row["x0_t"].to_bits().to_le_bytes());
            for field in state_fields(orientation_mode) {
                hasher.update(row[&input_state_key(field)].to_bits().to_le_bytes());
            }
            for input_step in 0..HORIZON {
                hasher.update(row[&motor_sequence_time_key(input_step)].to_bits().to_le_bytes());
                for field in MOTOR_INPUT_FIELDS {
                    hasher.update(row[&motor_sequence_key(field, input_step)].to_bits().to_le_bytes());
                }
            }
        }
        Some(format!("{:x}", hasher.finalize()))
    } else {
        None
    };

    Ok(RunInput {
        filename,
        run_id,
        sha256,
        ground_truth_window_sha256,
        conditioning_window_sha256,
        orientation_mode,
        rows,
        has_complete_window_truth,
    })
}

fn aggregate_runs(runs: &[RunInput], complete_windows: bool) -> Value {
    let scored = runs.iter().map(|run| score_run(run, complete_windows)).collect::<Vec<_>>();
    let mut per_horizon = Vec::with_capacity(HORIZON);
    let mut cumulative = Metrics::zero();

    for horizon in 1..=HORIZON {
        let mut sum = Metrics::zero();
        for run_score in &scored {
            let metric_value = &run_score["per_horizon"][horizon - 1]["metrics"];
            let metrics = Metrics {
                position_m: metric_value["position_mean_euclidean_error_m"].as_f64().unwrap(),
                velocity_m_s: metric_value["linear_velocity_mean_euclidean_error_m_s"].as_f64().unwrap(),
                orientation_rad: metric_value["orientation_geodesic_error_rad_reference_formula"].as_f64().unwrap(),
                orientation_sign_invariant_rad: metric_value["orientation_geodesic_error_rad_sign_invariant"].as_f64().unwrap(),
                angular_rate_rad_s: metric_value["body_angular_rate_mean_euclidean_error_rad_s"].as_f64().unwrap(),
            };
            sum.add_assign(metrics);
        }
        let average = sum.scale(1.0 / runs.len() as f64);
        cumulative.add_assign(average);
        per_horizon.push(json!({
            "horizon_steps": horizon,
            "independent_run_count": runs.len(),
            "macro_mean_over_runs": average.to_json(),
            "run_values_are_retained_not_pooled": true
        }));
    }

    json!({
        "aggregation": "macro-mean independent per-run metrics; no windows or timestamp rows pooled across run boundaries",
        "per_horizon": per_horizon,
        "macro_mean_simulation_error_sum_over_horizons_1_to_50": cumulative.to_json()
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(invalid_input(
            "usage: nanodrone_metric_receipt <melon_prediction_run1.csv> <melon_prediction_run2.csv> <melon_prediction_run3.csv>",
        ).into());
    }

    let mut runs = args.iter().map(|arg| load_run(Path::new(arg))).collect::<Result<Vec<_>, _>>()?;
    runs.sort_by(|a, b| a.run_id.cmp(&b.run_id));
    let run_ids = runs.iter().map(|run| run.run_id.as_str()).collect::<Vec<_>>();
    if run_ids.len() != 3 || run_ids[0] != "run1" || run_ids[1] != "run2" || run_ids[2] != "run3" {
        return Err(invalid_input(format!(
            "expected exactly one file for each held-out Melon run1/run2/run3; received {run_ids:?}"
        )).into());
    }
    let orientation_modes = runs.iter().map(|run| run.orientation_mode).collect::<HashSet<_>>();
    if orientation_modes.len() != 1 {
        return Err(invalid_input("all test runs must use one shared orientation representation").into());
    }
    let complete_truth_flags = runs.iter().map(|run| run.has_complete_window_truth).collect::<HashSet<_>>();
    if complete_truth_flags.len() != 1 {
        return Err(invalid_input("all test runs must share the same per-horizon target schema").into());
    }

    let per_run = runs.iter().map(|run| json!({
        "file": run.filename,
        "run_id": run.run_id,
        "sha256": run.sha256,
        "ground_truth_window_sha256": run.ground_truth_window_sha256,
        "conditioning_window_sha256": run.conditioning_window_sha256,
        "row_count": run.rows.len(),
        "orientation_representation": run.orientation_mode.label(),
        "legacy_shifted_truth_metric": score_run(run, false),
        "complete_window_metric": if run.has_complete_window_truth {
            score_run(run, true)
        } else {
            Value::Null
        }
    })).collect::<Vec<_>>();

    let macro_mean_complete = if runs[0].has_complete_window_truth {
        aggregate_runs(&runs, true)
    } else {
        Value::Null
    };
    println!("{}", serde_json::to_string_pretty(&json!({
        "benchmark_id": "symtropy-nanodrone-multihorizon-metrics-v0.1",
        "claim_ceiling": "real_dataset_prediction_metrics_only_not_control_or_safety_evidence",
        "source": {
            "benchmark": "IDSIA Nano-Quadrotor System Identification Benchmark",
            "metric_reference": "nonlinear_benchmarks/nanodrone_error_metrics.py",
            "alignment_note": "source-compatible shifted-truth formula is computed independently per run; all-available-window scoring requires explicit per-horizon truth columns"
        },
        "protocol": {
            "held_out_trajectory": "melon",
            "required_runs": ["run1", "run2", "run3"],
            "required_horizons": [1, 5, 10, 20, 50],
            "evaluated_horizons": HORIZON,
            "sample_period_s": EXPECTED_DT_S,
            "ground_truth_pairing": "SHA-256 over each row timestamp and all h=1..50 target state values in fixed state-field/horizon order, hashing parsed IEEE-754 f64 bits in little-endian order; exact match required for cross-model pairing",
            "conditioning_pairing": "SHA-256 over raw input state x0, each of the 50 source timestamps, and the complete four-channel 50-step motor-input sequence u_seq for every prediction window; scalar motor columns cannot certify identical conditioning",
            "metric_definitions": {
                "position": "mean Euclidean norm of 3D position error, metres",
                "linear_velocity": "mean Euclidean norm of 3D world-frame velocity error, metres/second",
                "orientation": "publish both the exact source 2*atan2 relative-quaternion formula and a sign-invariant normalized SO(3) geodesic diagnostic, radians",
                "body_angular_rate": "mean Euclidean norm of 3D body-frame angular-rate error, radians/second",
                "simulation_error": "sum of the per-horizon mean errors from h=1 through h=50"
            },
            "independent_run_boundaries_preserved": true,
            "missing_and_non_finite_required_values": "fail closed, never silently dropped",
            "orientation_sign_canonicalization": "none in reference-compatible metric; preserves upstream formula exactly"
        },
        "per_run": per_run,
        "macro_mean_reference_formula_per_run": aggregate_runs(&runs, false),
        "macro_mean_all_available_windows": macro_mean_complete
    }))?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditioning_timestamps_reject_gaps_and_x0_mismatch() {
        let mut row = HashMap::new();
        row.insert("x0_t".to_owned(), 0.0);
        for step in 0..HORIZON {
            row.insert(
                motor_sequence_time_key(step),
                step as f64 * EXPECTED_DT_S,
            );
        }
        assert!(validate_conditioning_timestamps(&row, "regular fixture").is_ok());

        row.insert(motor_sequence_time_key(49), 0.9);
        assert!(validate_conditioning_timestamps(&row, "gapped fixture").is_err());

        row.insert(motor_sequence_time_key(49), 0.49);
        row.insert("x0_t".to_owned(), 0.001);
        assert!(validate_conditioning_timestamps(&row, "x0 mismatch fixture").is_err());
    }

    #[test]
    fn rotation_vector_maps_to_expected_quarter_turn_quaternion() {
        let q = rotvec_to_quaternion([0.0, 0.0, std::f64::consts::FRAC_PI_2]);
        assert!((q[2] - (std::f64::consts::FRAC_PI_4).sin()).abs() < 1.0e-12);
        assert!((q[3] - (std::f64::consts::FRAC_PI_4).cos()).abs() < 1.0e-12);
        assert!((quaternion_geodesic_error([0.0, 0.0, 0.0, 1.0], q)
            - std::f64::consts::FRAC_PI_2).abs() < 1.0e-12);
    }

    #[test]
    fn antipodal_quaternions_are_same_rotation_in_sign_invariant_metric() {
        let truth = [0.0, 0.0, 0.0, 1.0];
        let prediction = [0.0, 0.0, 0.0, -1.0];
        assert!((quaternion_geodesic_error(truth, prediction) - 2.0 * std::f64::consts::PI).abs() < 1.0e-12);
        assert!(quaternion_geodesic_error_sign_invariant(truth, prediction).abs() < 1.0e-12);
    }

    #[test]
    fn euclidean_component_error_matches_reference_vector_norm() {
        let mut truth = HashMap::new();
        let mut prediction = HashMap::new();
        for (field, truth_value, predicted_value) in [
            ("x", 0.0, 3.0), ("y", 0.0, 4.0), ("z", 0.0, 0.0),
        ] {
            truth.insert(field.to_owned(), truth_value);
            prediction.insert(prediction_key(field, 1), predicted_value);
        }
        assert!((euclidean_error(&truth, &prediction, &POSITION_FIELDS, None, 1) - 5.0).abs() < 1.0e-12);
    }

    #[test]
    fn shifted_truth_alignment_uses_h_minus_one_rows_but_never_crosses_runs() {
        let row_count = 100usize;
        let horizon = 50usize;
        let legacy_count = row_count - (horizon - 1);
        assert_eq!(legacy_count, 51);
        let first_truth_row = 0usize + horizon - 1;
        assert_eq!(first_truth_row, 49);
        // Scoring is called on each RunInput before aggregation; a truth index can never
        // address the next independent run because the input's row vector is run-local.
    }

    #[test]
    fn filename_must_name_one_of_the_three_held_out_runs() {
        assert_eq!(
            run_id_from_filename(Path::new("/results/melon_20251017_run1.csv")).unwrap().1,
            "run1"
        );
        assert!(run_id_from_filename(Path::new("/results/melon_run10.csv")).is_err());
        assert!(run_id_from_filename(Path::new("/results/square_run1.csv")).is_err());
    }

    #[test]
    fn run_file_scores_all_complete_windows_and_reports_shifted_truth_truncation() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("symtropy-nanodrone-metrics-{nonce}"));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("melon_20251017_run1.csv");

        let mode = OrientationMode::RotationVector;
        let fields = state_fields(mode);
        let mut headers = vec!["t".to_owned()];
        headers.extend(fields.iter().map(|field| (*field).to_owned()));
        for horizon in 1..=HORIZON {
            headers.extend(fields.iter().map(|field| prediction_key(field, horizon)));
        }
        for horizon in 1..=HORIZON {
            headers.extend(fields.iter().map(|field| target_key(field, horizon)));
        }
        headers.push("x0_t".to_owned());
        headers.extend(fields.iter().map(|field| input_state_key(field)));
        headers.extend((0..HORIZON).map(motor_sequence_time_key));
        headers.extend(MOTOR_INPUT_FIELDS.iter().flat_map(|field|
            (0..HORIZON).map(move |step| motor_sequence_key(field, step))
        ));

        {
            let mut writer = csv::Writer::from_path(&path).unwrap();
            writer.write_record(&headers).unwrap();
            for row_index in 0..HORIZON {
                let mut row = Vec::with_capacity(headers.len());
                row.push((row_index as f64 * EXPECTED_DT_S).to_string());
                row.extend(fields.iter().map(|_| "0".to_owned()));
                for _ in 1..=HORIZON {
                    row.extend(fields.iter().map(|_| "0".to_owned()));
                }
                for _ in 1..=HORIZON {
                    row.extend(fields.iter().map(|_| "0".to_owned()));
                }
                row.push("0".to_owned());
                row.extend(fields.iter().map(|_| "0".to_owned()));
                for input_step in 0..HORIZON {
                    row.push(format!("{:.8}", input_step as f64 * EXPECTED_DT_S));
                }
                for (field_index, _) in MOTOR_INPUT_FIELDS.iter().enumerate() {
                    for input_step in 0..HORIZON {
                        row.push(format!("{}", field_index * 100 + input_step));
                    }
                }
                writer.write_record(&row).unwrap();
            }
            writer.flush().unwrap();
        }

        let run = load_run(&path).unwrap();
        assert!(run.has_complete_window_truth);
        assert!(run.ground_truth_window_sha256.is_some());
        assert!(run.conditioning_window_sha256.is_some());

        // Change one motor sample while preserving targets and x0. The conditioning
        // digest must change, otherwise model-input parity would not be attested.
        let altered_path = directory.join("melon_20251017_run2.csv");
        {
            let bytes = fs::read(&path).unwrap();
            let mut reader = ReaderBuilder::new().has_headers(true).from_reader(bytes.as_slice());
            let headers = reader.headers().unwrap().clone();
            let changed_column = headers.iter()
                .position(|header| header == motor_sequence_key("m2_rads", 17))
                .unwrap();
            let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
            let mut writer = csv::Writer::from_path(&altered_path).unwrap();
            writer.write_record(&headers).unwrap();
            for (row_index, record) in records.iter().enumerate() {
                let mut values = record.iter().map(str::to_owned).collect::<Vec<_>>();
                if row_index == 0 {
                    values[changed_column] = "999.0".to_owned();
                }
                writer.write_record(&values).unwrap();
            }
            writer.flush().unwrap();
        }
        let altered = load_run(&altered_path).unwrap();
        assert_ne!(run.conditioning_window_sha256, altered.conditioning_window_sha256);
        assert_eq!(run.ground_truth_window_sha256, altered.ground_truth_window_sha256);

        let shifted = score_run(&run, false);
        let complete = score_run(&run, true);
        assert_eq!(shifted["per_horizon"][0]["sample_count"], 50);
        assert_eq!(shifted["per_horizon"][49]["sample_count"], 1);
        assert_eq!(
            shifted["per_horizon"][49]["prediction_windows_not_scored_by_shifted_truth_alignment"],
            49
        );
        assert_eq!(complete["per_horizon"][49]["sample_count"], 50);
        assert_eq!(
            complete["per_horizon"][49]["prediction_windows_not_scored_by_shifted_truth_alignment"],
            0
        );

        fs::remove_file(&path).unwrap();
        fs::remove_dir(&directory).unwrap();
    }

    #[test]
    fn simerr_is_a_sum_not_an_average_over_horizons() {
        let one_per_horizon = Metrics {
            position_m: 1.0,
            velocity_m_s: 2.0,
            orientation_rad: 3.0,
            orientation_sign_invariant_rad: 0.5,
            angular_rate_rad_s: 4.0,
        };
        let summed = one_per_horizon.scale(HORIZON as f64);
        assert_eq!(summed.position_m, 50.0);
        assert_eq!(summed.velocity_m_s, 100.0);
        assert_eq!(summed.orientation_sign_invariant_rad, 25.0);
    }
}
