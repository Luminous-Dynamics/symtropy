#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use std::{env, fs, process};

use sha2::{Digest, Sha256};

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EPISODES: usize = 100_000;
const MAX_FRAMES_PER_EPISODE: usize = 1_000_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateManifest {
    schema_version: u32,
    benchmark_id: String,
    episodes: Vec<CandidateEpisode>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateEpisode {
    episode_id: String,
    input_bundle_path: String,
    input_bundle_sha256: String,
    frame_budget: usize,
    expected_frame_count: usize,
    first_sequence: u64,
    horizon_us: u64,
    frames: Vec<CandidateFrame>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateFrame {
    frame_id: String,
    sequence: u64,
    timestamp_us: u64,
    modalities: Vec<Modality>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Modality {
    Rgb,
    Event,
    Depth,
    Infrared,
    Lidar,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluatorLedger {
    schema_version: u32,
    benchmark_id: String,
    episodes: Vec<LedgerEpisode>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerEpisode {
    episode_id: String,
    parent_episode_id: String,
    counterfactual_group_id: String,
    variant_kind: VariantKind,
    split: Split,
    generalization: GeneralizationStratum,
    independent_unit: IndependentUnit,
    fault_ids: Vec<String>,
    fault_families: Vec<FaultFamily>,
    critical: bool,
    expected_disposition: Disposition,
    ground_truth_pose: PoseGroundTruth,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct IndependentUnit {
    target_id: String,
    session_id: String,
    trajectory_id: String,
    sensor_rig_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Split {
    Development,
    Calibration,
    Confirmatory,
    BlindHoldout,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum GeneralizationStratum {
    SeenTarget,
    UnseenTarget,
    UnseenTargetFamily,
    UnseenRenderer,
    UnseenSensorRig,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum VariantKind {
    Clean,
    SingleFault,
    Interaction,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum FaultFamily {
    RgbLocal,
    EventLocal,
    Timing,
    Extrinsics,
    CommonCause,
    Translation,
    Dynamics,
    Observability,
    Integrity,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    Qualified,
    Abstain,
    OutOfScope,
    Disagreement,
    DependenceUnknown,
    MissingEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PredictionFile {
    schema_version: u32,
    benchmark_id: String,
    predictions: Vec<Prediction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoseGroundTruth {
    coordinate_frame: String,
    transform_direction: String,
    translation_m: [f64; 3],
    orientation_xyzw: [f64; 4],
    annotation_sha256: String,
    label_audit_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoseEstimate {
    coordinate_frame: String,
    transform_direction: String,
    translation_m: [f64; 3],
    orientation_xyzw: [f64; 4],
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoseRegion {
    coordinate_frame: String,
    transform_direction: String,
    translation_lower_m: [f64; 3],
    translation_upper_m: [f64; 3],
    orientation_center_xyzw: [f64; 4],
    orientation_half_angle_rad: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Prediction {
    episode_id: String,
    input_bundle_sha256: String,
    disposition: Disposition,
    detected_fault_families: Vec<FaultFamily>,
    pose: Option<PoseEstimate>,
    uncertainty_region: Option<PoseRegion>,
    orientation_method: Option<String>,
    translation_method: Option<String>,
}

#[derive(Debug, Serialize)]
struct Score {
    benchmark_id: String,
    episodes: usize,
    independent_units: usize,
    counterfactual_groups: usize,
    clean_episodes: usize,
    fault_episodes: usize,
    critical_fault_episodes: usize,
    critical_correctly_contained_faults: usize,
    critical_unsafe_accepts: usize,
    clean_qualified: usize,
    false_abstentions: usize,
    correctly_contained_faults: usize,
    unsafe_accepts: usize,
    exact_disposition_matches: usize,
    false_positive_family_labels: usize,
    family_detection: Vec<FamilyMetric>,
    generalization: Vec<GeneralizationMetric>,
    pose_estimates: usize,
    pose_region_covered: usize,
    pose_silent_misses: usize,
    pose_metrics: Vec<PoseMetric>,
    pose_summary: Option<PoseSummary>,
}

#[derive(Debug, Serialize)]
struct FamilyMetric {
    family: FaultFamily,
    episodes: usize,
    detected: usize,
}

#[derive(Debug, Serialize)]
struct PoseMetric {
    episode_id: String,
    translation_error_m: f64,
    relative_translation_error: Option<f64>,
    rotation_error_rad: f64,
    rotation_error_deg: f64,
    uncertainty_region_covered: bool,
}

#[derive(Debug, Serialize)]
struct PoseSummary {
    estimates: usize,
    region_coverage_denominator: usize,
    region_covered: usize,
    silent_misses: usize,
    median_translation_error_m: f64,
    p95_translation_error_m: f64,
    median_relative_translation_error: Option<f64>,
    p95_relative_translation_error: Option<f64>,
    median_rotation_error_rad: f64,
    p95_rotation_error_rad: f64,
    median_rotation_error_deg: f64,
    p95_rotation_error_deg: f64,
}

#[derive(Debug, Serialize)]
struct GeneralizationMetric {
    stratum: GeneralizationStratum,
    episodes: usize,
    fault_episodes: usize,
    critical_fault_episodes: usize,
    unsafe_accepts: usize,
    clean_episodes: usize,
    clean_qualified: usize,
    false_abstentions: usize,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn verify_input_bundle(bundle_root: &Path, episode: &CandidateEpisode) -> Result<(), String> {
    let relative = Path::new(&episode.input_bundle_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!(
            "episode {} has an unsafe input_bundle_path",
            episode.episode_id
        ));
    }

    let root = fs::canonicalize(bundle_root).map_err(|error| {
        format!(
            "cannot resolve bundle root {}: {error}",
            bundle_root.display()
        )
    })?;
    let resolved = fs::canonicalize(root.join(relative)).map_err(|error| {
        format!(
            "cannot resolve bundle for episode {} at {}: {error}",
            episode.episode_id,
            relative.display()
        )
    })?;
    if !resolved.starts_with(&root) {
        return Err(format!(
            "episode {} bundle path escapes the declared root",
            episode.episode_id
        ));
    }

    let metadata = fs::metadata(&resolved)
        .map_err(|error| format!("cannot stat bundle {}: {error}", resolved.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "episode {} bundle is not a regular file or exceeds {} bytes",
            episode.episode_id, MAX_INPUT_BYTES
        ));
    }
    let bytes = fs::read(&resolved)
        .map_err(|error| format!("cannot read bundle {}: {error}", resolved.display()))?;
    let byte_count = u64::try_from(bytes.len())
        .map_err(|_| format!("episode {} bundle length exceeds u64", episode.episode_id))?;
    if byte_count > MAX_INPUT_BYTES {
        return Err(format!(
            "episode {} bundle grew beyond the byte limit",
            episode.episode_id
        ));
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != episode.input_bundle_sha256 {
        return Err(format!(
            "episode {} source bundle SHA-256 mismatch: expected {}, observed {}",
            episode.episode_id, episode.input_bundle_sha256, actual
        ));
    }

    // The non-image contract fixture cross-binds the episode header and every
    // frame identity/time/modality row to the manifest. This prevents a valid
    // source-byte digest from being paired with substituted metadata.
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        format!(
            "episode {} bundle is not UTF-8: {error}",
            episode.episode_id
        )
    })?;
    let mut lines = text.lines();
    let expected_episode = format!("episode={}", episode.episode_id);
    let expected_metadata = format!(
        "frame_budget={};expected_frame_count={};first_sequence={};horizon_us={}",
        episode.frame_budget,
        episode.expected_frame_count,
        episode.first_sequence,
        episode.horizon_us
    );
    if lines.next() != Some("CFI_SPACE_SYNTHETIC_NON_IMAGE_PAYLOAD_V1")
        || lines.next() != Some(expected_episode.as_str())
        || lines.next() != Some(expected_metadata.as_str())
    {
        return Err(format!(
            "episode {} bundle header does not match manifest",
            episode.episode_id
        ));
    }

    for (index, frame) in episode.frames.iter().enumerate() {
        let modalities = frame
            .modalities
            .iter()
            .map(|modality| match modality {
                Modality::Rgb => "rgb",
                Modality::Event => "event",
                Modality::Depth => "depth",
                Modality::Infrared => "infrared",
                Modality::Lidar => "lidar",
            })
            .collect::<Vec<_>>()
            .join(",");
        let prefix = format!(
            "frame={index};frame_id={};sequence={};timestamp_us={};modalities={modalities};payload=",
            frame.frame_id, frame.sequence, frame.timestamp_us
        );
        let row = lines
            .next()
            .ok_or_else(|| format!("episode {} bundle omits frame {index}", episode.episode_id))?;
        if !row.starts_with(&prefix) || row.len() <= prefix.len() {
            return Err(format!(
                "episode {} bundle frame {index} metadata/payload mismatch",
                episode.episode_id
            ));
        }
    }
    if lines.next().is_some() {
        return Err(format!(
            "episode {} bundle has unmanifested trailing records",
            episode.episode_id
        ));
    }
    Ok(())
}

fn validate_candidate_manifest(
    manifest: &CandidateManifest,
    bundle_root: &Path,
) -> Result<(), String> {
    if manifest.schema_version != 1 {
        return Err(format!(
            "unsupported candidate manifest schema_version {}",
            manifest.schema_version
        ));
    }
    require_nonempty(&manifest.benchmark_id, "benchmark_id")?;
    if manifest.episodes.is_empty() || manifest.episodes.len() > MAX_EPISODES {
        return Err(format!(
            "episode count must be in 1..={MAX_EPISODES}, got {}",
            manifest.episodes.len()
        ));
    }

    let mut episode_ids = BTreeSet::new();
    for episode in &manifest.episodes {
        require_nonempty(&episode.episode_id, "episode_id")?;
        if !episode_ids.insert(&episode.episode_id) {
            return Err(format!("duplicate episode_id {}", episode.episode_id));
        }
        if !is_sha256_hex(&episode.input_bundle_sha256) {
            return Err(format!(
                "episode {} has malformed input_bundle_sha256",
                episode.episode_id
            ));
        }
        if episode.input_bundle_path.trim().is_empty() {
            return Err(format!(
                "episode {} has empty input_bundle_path",
                episode.episode_id
            ));
        }
        verify_input_bundle(bundle_root, episode)?;
        if episode.frame_budget == 0
            || episode.expected_frame_count == 0
            || episode.expected_frame_count > episode.frame_budget
            || episode.expected_frame_count != episode.frames.len()
            || episode.frames.len() > MAX_FRAMES_PER_EPISODE
        {
            return Err(format!(
                "episode {} has inconsistent frame budget/count",
                episode.episode_id
            ));
        }
        if episode.horizon_us == 0 {
            return Err(format!("episode {} has zero horizon", episode.episode_id));
        }

        let mut frame_ids = BTreeSet::new();
        let mut first_timestamp = None;
        let mut previous_timestamp = None;
        for (index, frame) in episode.frames.iter().enumerate() {
            require_nonempty(&frame.frame_id, "frame_id")?;
            if !frame_ids.insert(&frame.frame_id) {
                return Err(format!(
                    "episode {} contains duplicate frame_id {}",
                    episode.episode_id, frame.frame_id
                ));
            }
            let offset = u64::try_from(index).map_err(|_| "frame index exceeds u64".to_string())?;
            let expected_sequence = episode
                .first_sequence
                .checked_add(offset)
                .ok_or_else(|| "frame sequence overflow".to_string())?;
            if frame.sequence != expected_sequence {
                return Err(format!(
                    "episode {} frame {} sequence gap: expected {}, got {}",
                    episode.episode_id, frame.frame_id, expected_sequence, frame.sequence
                ));
            }
            if frame.modalities.is_empty() {
                return Err(format!(
                    "episode {} frame {} has no modality",
                    episode.episode_id, frame.frame_id
                ));
            }
            let unique_modalities: BTreeSet<_> = frame.modalities.iter().copied().collect();
            if unique_modalities.len() != frame.modalities.len() {
                return Err(format!(
                    "episode {} frame {} repeats a modality",
                    episode.episode_id, frame.frame_id
                ));
            }

            if let Some(previous) = previous_timestamp {
                if frame.timestamp_us <= previous {
                    return Err(format!(
                        "episode {} timestamps are not strictly increasing",
                        episode.episode_id
                    ));
                }
            } else {
                first_timestamp = Some(frame.timestamp_us);
            }
            previous_timestamp = Some(frame.timestamp_us);
        }

        let start = first_timestamp
            .ok_or_else(|| format!("episode {} has no first timestamp", episode.episode_id))?;
        let end = previous_timestamp
            .ok_or_else(|| format!("episode {} has no last timestamp", episode.episode_id))?;
        if end - start > episode.horizon_us {
            return Err(format!(
                "episode {} exceeds its declared fixed horizon",
                episode.episode_id
            ));
        }
    }
    Ok(())
}

fn finite_vec3(value: &[f64; 3], name: &str) -> Result<(), String> {
    if value.iter().any(|component| !component.is_finite()) {
        return Err(format!("{name} contains a non-finite value"));
    }
    Ok(())
}

fn quaternion_norm(value: &[f64; 4]) -> f64 {
    value.iter().map(|component| component * component).sum::<f64>().sqrt()
}

fn validate_quaternion(value: &[f64; 4], name: &str) -> Result<(), String> {
    if value.iter().any(|component| !component.is_finite()) {
        return Err(format!("{name} contains a non-finite component"));
    }
    let norm = quaternion_norm(value);
    if !norm.is_finite() || (norm - 1.0).abs() > 1e-3 {
        return Err(format!("{name} must be a unit quaternion (norm tolerance 1e-3)"));
    }
    Ok(())
}

fn quaternion_geodesic_rad(left: &[f64; 4], right: &[f64; 4]) -> f64 {
    let left_norm = quaternion_norm(left);
    let right_norm = quaternion_norm(right);
    let dot = left
        .iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum::<f64>()
        / (left_norm * right_norm);
    2.0 * dot.abs().clamp(0.0, 1.0).acos()
}

fn translation_distance(left: &[f64; 3], right: &[f64; 3]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

fn validate_ground_truth_pose(pose: &PoseGroundTruth, episode_id: &str) -> Result<(), String> {
    require_nonempty(&pose.coordinate_frame, "ground-truth coordinate_frame")?;
    require_nonempty(&pose.transform_direction, "ground-truth transform_direction")?;
    finite_vec3(&pose.translation_m, "ground-truth translation_m")?;
    validate_quaternion(&pose.orientation_xyzw, "ground-truth orientation_xyzw")?;
    if !is_sha256_hex(&pose.annotation_sha256) || !is_sha256_hex(&pose.label_audit_sha256) {
        return Err(format!(
            "episode {episode_id} ground-truth annotation/label-audit digest is malformed"
        ));
    }
    Ok(())
}

fn validate_pose_prediction(
    prediction: &Prediction,
    truth: &LedgerEpisode,
) -> Result<Option<PoseMetric>, String> {
    let qualified = prediction.disposition == Disposition::Qualified;
    match (
        qualified,
        prediction.pose.as_ref(),
        prediction.uncertainty_region.as_ref(),
        prediction.orientation_method.as_deref(),
        prediction.translation_method.as_deref(),
    ) {
        (true, Some(pose), Some(region), Some(orientation_method), Some(translation_method)) => {
            require_nonempty(orientation_method, "orientation_method")?;
            require_nonempty(translation_method, "translation_method")?;
            if pose.coordinate_frame != truth.ground_truth_pose.coordinate_frame
                || pose.transform_direction != truth.ground_truth_pose.transform_direction
                || region.coordinate_frame != truth.ground_truth_pose.coordinate_frame
                || region.transform_direction != truth.ground_truth_pose.transform_direction
            {
                return Err(format!(
                    "episode {} pose/region coordinate-frame or transform-direction mismatch",
                    prediction.episode_id
                ));
            }
            finite_vec3(&pose.translation_m, "pose translation_m")?;
            validate_quaternion(&pose.orientation_xyzw, "pose orientation_xyzw")?;
            finite_vec3(&region.translation_lower_m, "region translation_lower_m")?;
            finite_vec3(&region.translation_upper_m, "region translation_upper_m")?;
            validate_quaternion(
                &region.orientation_center_xyzw,
                "region orientation_center_xyzw",
            )?;
            if !region.orientation_half_angle_rad.is_finite()
                || !(0.0..=std::f64::consts::PI).contains(&region.orientation_half_angle_rad)
            {
                return Err(format!(
                    "episode {} region orientation half-angle must be in [0, pi]",
                    prediction.episode_id
                ));
            }
            for axis in 0..3 {
                if region.translation_lower_m[axis] > region.translation_upper_m[axis] {
                    return Err(format!(
                        "episode {} uncertainty translation lower bound exceeds upper bound on axis {axis}",
                        prediction.episode_id
                    ));
                }
            }

            let translation_error_m =
                translation_distance(&pose.translation_m, &truth.ground_truth_pose.translation_m);
            let ground_truth_range =
                truth.ground_truth_pose.translation_m.iter().map(|v| v * v).sum::<f64>().sqrt();
            let relative_translation_error = (ground_truth_range > 1e-9)
                .then_some(translation_error_m / ground_truth_range);
            let rotation_error_rad = quaternion_geodesic_rad(
                &pose.orientation_xyzw,
                &truth.ground_truth_pose.orientation_xyzw,
            );
            let translation_inside = (0..3).all(|axis| {
                truth.ground_truth_pose.translation_m[axis] >= region.translation_lower_m[axis]
                    && truth.ground_truth_pose.translation_m[axis]
                        <= region.translation_upper_m[axis]
            });
            let orientation_inside = quaternion_geodesic_rad(
                &region.orientation_center_xyzw,
                &truth.ground_truth_pose.orientation_xyzw,
            ) <= region.orientation_half_angle_rad + 1e-12;
            Ok(Some(PoseMetric {
                episode_id: prediction.episode_id.clone(),
                translation_error_m,
                relative_translation_error,
                rotation_error_rad,
                rotation_error_deg: rotation_error_rad.to_degrees(),
                uncertainty_region_covered: translation_inside && orientation_inside,
            }))
        }
        (false, None, None, None, None) => Ok(None),
        _ => Err(format!(
            "episode {} must emit a pose, region and both component method IDs iff disposition is qualified",
            prediction.episode_id
        )),
    }
}

fn quantile(values: &mut [f64], probability: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    if values.len() == 1 {
        return Some(values[0]);
    }
    let position = probability * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let weight = position - lower as f64;
    Some(values[lower] * (1.0 - weight) + values[upper] * weight)
}

fn summarize_pose_metrics(metrics: &[PoseMetric]) -> Option<PoseSummary> {
    if metrics.is_empty() {
        return None;
    }
    let mut translation: Vec<_> = metrics.iter().map(|row| row.translation_error_m).collect();
    let mut relative: Vec<_> = metrics.iter().filter_map(|row| row.relative_translation_error).collect();
    let mut rotation_rad: Vec<_> = metrics.iter().map(|row| row.rotation_error_rad).collect();
    let mut rotation_deg: Vec<_> = metrics.iter().map(|row| row.rotation_error_deg).collect();
    let region_covered = metrics.iter().filter(|row| row.uncertainty_region_covered).count();
    Some(PoseSummary {
        estimates: metrics.len(),
        region_coverage_denominator: metrics.len(),
        region_covered,
        silent_misses: metrics.len() - region_covered,
        median_translation_error_m: quantile(&mut translation, 0.5)?,
        p95_translation_error_m: quantile(&mut translation, 0.95)?,
        median_relative_translation_error: quantile(&mut relative, 0.5),
        p95_relative_translation_error: quantile(&mut relative, 0.95),
        median_rotation_error_rad: quantile(&mut rotation_rad, 0.5)?,
        p95_rotation_error_rad: quantile(&mut rotation_rad, 0.95)?,
        median_rotation_error_deg: quantile(&mut rotation_deg, 0.5)?,
        p95_rotation_error_deg: quantile(&mut rotation_deg, 0.95)?,
    })
}

fn validate_ledger(ledger: &EvaluatorLedger) -> Result<(), String> {
    if ledger.schema_version != 1 {
        return Err(format!(
            "unsupported evaluator ledger schema_version {}",
            ledger.schema_version
        ));
    }
    require_nonempty(&ledger.benchmark_id, "ledger benchmark_id")?;
    if ledger.episodes.is_empty() || ledger.episodes.len() > MAX_EPISODES {
        return Err(format!(
            "ledger episode count must be in 1..={MAX_EPISODES}, got {}",
            ledger.episodes.len()
        ));
    }

    let mut episode_ids = BTreeSet::new();
    let mut groups: BTreeMap<String, (String, Split, IndependentUnit)> = BTreeMap::new();
    let mut group_variants: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut target_splits = BTreeMap::new();
    let mut session_splits = BTreeMap::new();
    let mut trajectory_splits = BTreeMap::new();
    let mut rig_splits = BTreeMap::new();

    for row in &ledger.episodes {
        for (field, value) in [
            ("episode_id", row.episode_id.as_str()),
            ("parent_episode_id", row.parent_episode_id.as_str()),
            (
                "counterfactual_group_id",
                row.counterfactual_group_id.as_str(),
            ),
            ("target_id", row.independent_unit.target_id.as_str()),
            ("session_id", row.independent_unit.session_id.as_str()),
            ("trajectory_id", row.independent_unit.trajectory_id.as_str()),
            ("sensor_rig_id", row.independent_unit.sensor_rig_id.as_str()),
        ] {
            require_nonempty(value, field)?;
        }
        if !episode_ids.insert(&row.episode_id) {
            return Err(format!("duplicate evaluator episode_id {}", row.episode_id));
        }
        validate_ground_truth_pose(&row.ground_truth_pose, &row.episode_id)?;

        let fault_ids: BTreeSet<_> = row.fault_ids.iter().collect();
        let fault_families: BTreeSet<_> = row.fault_families.iter().collect();
        if fault_ids.len() != row.fault_ids.len()
            || fault_families.len() != row.fault_families.len()
            || row.fault_ids.iter().any(|id| id.trim().is_empty())
        {
            return Err(format!(
                "episode {} has duplicate or empty fault identifiers",
                row.episode_id
            ));
        }

        match row.variant_kind {
            VariantKind::Clean => {
                if !row.fault_ids.is_empty()
                    || !row.fault_families.is_empty()
                    || row.expected_disposition != Disposition::Qualified
                {
                    return Err(format!(
                        "clean episode {} must have no faults and expect qualified",
                        row.episode_id
                    ));
                }
            }
            VariantKind::SingleFault => {
                if row.fault_ids.len() != 1
                    || row.fault_families.len() != 1
                    || row.expected_disposition == Disposition::Qualified
                {
                    return Err(format!(
                        "single-fault episode {} must declare exactly one fault and be non-qualified",
                        row.episode_id
                    ));
                }
            }
            VariantKind::Interaction => {
                if row.fault_ids.len() < 2
                    || row.fault_families.is_empty()
                    || row.expected_disposition == Disposition::Qualified
                {
                    return Err(format!(
                        "interaction episode {} must declare at least two faults and be non-qualified",
                        row.episode_id
                    ));
                }
            }
        }

        let counts = group_variants
            .entry(row.counterfactual_group_id.clone())
            .or_insert((0, 0));
        match row.variant_kind {
            VariantKind::Clean => counts.0 += 1,
            VariantKind::SingleFault | VariantKind::Interaction => counts.1 += 1,
        }

        if let Some((parent, split, unit)) = groups.get(&row.counterfactual_group_id) {
            if parent != &row.parent_episode_id
                || split != &row.split
                || unit != &row.independent_unit
            {
                return Err(format!(
                    "counterfactual siblings in group {} cross parent, split or independent-unit boundaries",
                    row.counterfactual_group_id
                ));
            }
        } else {
            groups.insert(
                row.counterfactual_group_id.clone(),
                (
                    row.parent_episode_id.clone(),
                    row.split,
                    row.independent_unit.clone(),
                ),
            );
        }

        for (name, value, map) in [
            (
                "target",
                row.independent_unit.target_id.as_str(),
                &mut target_splits,
            ),
            (
                "session",
                row.independent_unit.session_id.as_str(),
                &mut session_splits,
            ),
            (
                "trajectory",
                row.independent_unit.trajectory_id.as_str(),
                &mut trajectory_splits,
            ),
            (
                "sensor rig",
                row.independent_unit.sensor_rig_id.as_str(),
                &mut rig_splits,
            ),
        ] {
            if let Some(existing) = map.insert(value.to_string(), row.split)
                && existing != row.split
            {
                return Err(format!("{name} {value} leaks across dataset splits"));
            }
        }
    }

    for (group, (clean_count, variant_count)) in group_variants {
        if clean_count != 1 || variant_count == 0 {
            return Err(format!(
                "counterfactual group {group} must contain exactly one clean episode and at least one fault variant"
            ));
        }
    }
    Ok(())
}

fn validate_and_score(
    manifest: &CandidateManifest,
    ledger: &EvaluatorLedger,
    predictions: &PredictionFile,
    bundle_root: &Path,
) -> Result<Score, String> {
    validate_candidate_manifest(manifest, bundle_root)?;
    validate_ledger(ledger)?;

    if predictions.schema_version != 2 {
        return Err(format!(
            "unsupported predictions schema_version {}",
            predictions.schema_version
        ));
    }
    if manifest.benchmark_id != ledger.benchmark_id
        || manifest.benchmark_id != predictions.benchmark_id
    {
        return Err("benchmark_id mismatch across manifest, ledger and predictions".to_string());
    }
    if predictions.predictions.len() != manifest.episodes.len() {
        return Err("prediction count does not match candidate episode count".to_string());
    }

    let manifest_ids: BTreeSet<_> = manifest
        .episodes
        .iter()
        .map(|episode| episode.episode_id.as_str())
        .collect();
    let ledger_ids: BTreeSet<_> = ledger
        .episodes
        .iter()
        .map(|episode| episode.episode_id.as_str())
        .collect();
    let prediction_ids: BTreeSet<_> = predictions
        .predictions
        .iter()
        .map(|prediction| prediction.episode_id.as_str())
        .collect();
    if manifest_ids != ledger_ids {
        return Err(
            "candidate manifest and evaluator ledger episode identities differ".to_string(),
        );
    }
    if manifest_ids != prediction_ids {
        return Err("candidate manifest and prediction episode identities differ".to_string());
    }
    if prediction_ids.len() != predictions.predictions.len() {
        return Err("duplicate prediction episode_id".to_string());
    }

    let candidate_by_id: BTreeMap<_, _> = manifest
        .episodes
        .iter()
        .map(|episode| (episode.episode_id.as_str(), episode))
        .collect();
    let ledger_by_id: BTreeMap<_, _> = ledger
        .episodes
        .iter()
        .map(|episode| (episode.episode_id.as_str(), episode))
        .collect();
    let prediction_by_id: BTreeMap<_, _> = predictions
        .predictions
        .iter()
        .map(|prediction| (prediction.episode_id.as_str(), prediction))
        .collect();

    let mut unsafe_accepts = 0;
    let mut false_abstentions = 0;
    let mut correctly_contained_faults = 0;
    let mut clean_qualified = 0;
    let mut fault_episodes = 0;
    let mut critical_fault_episodes = 0;
    let mut critical_correctly_contained_faults = 0;
    let mut critical_unsafe_accepts = 0;
    let mut clean_episodes = 0;
    let mut exact_disposition_matches = 0;
    let mut false_positive_family_labels = 0;
    let mut family_counts: BTreeMap<FaultFamily, (usize, usize)> = BTreeMap::new();
    let mut generalization_counts: BTreeMap<GeneralizationStratum, [usize; 7]> = BTreeMap::new();
    let mut pose_metrics = Vec::new();

    for episode_id in &manifest_ids {
        let episode = candidate_by_id[episode_id];
        let truth = ledger_by_id[episode_id];
        let prediction = prediction_by_id[episode_id];

        if prediction.input_bundle_sha256 != episode.input_bundle_sha256
            || !is_sha256_hex(&prediction.input_bundle_sha256)
        {
            return Err(format!(
                "episode {} prediction is not bound to the candidate input digest",
                episode_id
            ));
        }

        let predicted_families: BTreeSet<_> =
            prediction.detected_fault_families.iter().copied().collect();
        if predicted_families.len() != prediction.detected_fault_families.len() {
            return Err(format!(
                "episode {} repeats a detected fault family",
                episode_id
            ));
        }
        let expected_families: BTreeSet<_> = truth.fault_families.iter().copied().collect();
        if let Some(pose_metric) = validate_pose_prediction(prediction, truth)? {
            pose_metrics.push(pose_metric);
        }

        for family in &predicted_families {
            if !expected_families.contains(family) {
                false_positive_family_labels += 1;
            }
        }
        for family in &expected_families {
            let counts = family_counts.entry(*family).or_insert((0, 0));
            counts.0 += 1;
            if predicted_families.contains(family) {
                counts.1 += 1;
            }
        }

        let expected_qualified = truth.expected_disposition == Disposition::Qualified;
        let predicted_qualified = prediction.disposition == Disposition::Qualified;
        if truth.expected_disposition == prediction.disposition {
            exact_disposition_matches += 1;
        }
        let stratum_counts = generalization_counts
            .entry(truth.generalization)
            .or_insert([0; 7]);
        stratum_counts[0] += 1;

        if expected_qualified {
            clean_episodes += 1;
            stratum_counts[4] += 1;
            if predicted_qualified {
                clean_qualified += 1;
                stratum_counts[5] += 1;
            } else {
                false_abstentions += 1;
                stratum_counts[6] += 1;
            }
        } else {
            fault_episodes += 1;
            stratum_counts[1] += 1;
            if truth.critical {
                stratum_counts[2] += 1;
            }
            if truth.critical {
                critical_fault_episodes += 1;
            }
            if predicted_qualified {
                unsafe_accepts += 1;
                stratum_counts[3] += 1;
                if truth.critical {
                    critical_unsafe_accepts += 1;
                }
            } else {
                correctly_contained_faults += 1;
                if truth.critical {
                    critical_correctly_contained_faults += 1;
                }
            }
        }

        // Read the episode identity through the same validated map: this also
        // prevents accidental dead-code removal of the per-episode join.
        if episode.episode_id != *episode_id {
            return Err("internal episode identity join failure".to_string());
        }
    }

    let family_detection = family_counts
        .into_iter()
        .map(|(family, (episodes, detected))| FamilyMetric {
            family,
            episodes,
            detected,
        })
        .collect();

    let generalization = generalization_counts
        .into_iter()
        .map(|(stratum, counts)| GeneralizationMetric {
            stratum,
            episodes: counts[0],
            fault_episodes: counts[1],
            critical_fault_episodes: counts[2],
            unsafe_accepts: counts[3],
            clean_episodes: counts[4],
            clean_qualified: counts[5],
            false_abstentions: counts[6],
        })
        .collect();

    let independent_units: BTreeSet<_> = ledger
        .episodes
        .iter()
        .map(|row| {
            (
                row.independent_unit.target_id.as_str(),
                row.independent_unit.session_id.as_str(),
                row.independent_unit.trajectory_id.as_str(),
                row.independent_unit.sensor_rig_id.as_str(),
            )
        })
        .collect();

    let groups: BTreeSet<_> = ledger
        .episodes
        .iter()
        .map(|row| row.counterfactual_group_id.as_str())
        .collect();

    Ok(Score {
        benchmark_id: manifest.benchmark_id.clone(),
        episodes: manifest.episodes.len(),
        independent_units: independent_units.len(),
        counterfactual_groups: groups.len(),
        clean_episodes,
        fault_episodes,
        critical_fault_episodes,
        critical_correctly_contained_faults,
        critical_unsafe_accepts,
        clean_qualified,
        false_abstentions,
        correctly_contained_faults,
        unsafe_accepts,
        exact_disposition_matches,
        false_positive_family_labels,
        family_detection,
        generalization,
        pose_estimates: pose_metrics.len(),
        pose_region_covered: pose_metrics.iter().filter(|row| row.uncertainty_region_covered).count(),
        pose_silent_misses: pose_metrics.iter().filter(|row| !row.uncertainty_region_covered).count(),
        pose_summary: summarize_pose_metrics(&pose_metrics),
        pose_metrics,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("cannot stat {}: {error}", path.display()))?;
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "input {} exceeds {} bytes",
            path.display(),
            MAX_INPUT_BYTES
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let mut manifest_path = None;
    let mut ledger_path = None;
    let mut predictions_path = None;
    let mut expected_unsafe = None;
    let mut bundle_root = None;

    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            println!(
                "cfi-space-bench --manifest FILE --ledger FILE --predictions FILE --expect-unsafe N --bundle-root DIR"
            );
            return Ok(());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for argument {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = Some(value),
            "--ledger" => ledger_path = Some(value),
            "--predictions" => predictions_path = Some(value),
            "--expect-unsafe" => {
                expected_unsafe = Some(
                    value
                        .parse::<usize>()
                        .map_err(|error| format!("invalid --expect-unsafe value: {error}"))?,
                )
            }
            "--bundle-root" => bundle_root = Some(value),
            _ => return Err(format!("unknown argument {flag}")),
        }
    }

    let manifest_path = manifest_path.ok_or_else(|| "missing --manifest".to_string())?;
    let ledger_path = ledger_path.ok_or_else(|| "missing --ledger".to_string())?;
    let predictions_path = predictions_path.ok_or_else(|| "missing --predictions".to_string())?;
    let expected_unsafe = expected_unsafe.ok_or_else(|| "missing --expect-unsafe".to_string())?;
    let bundle_root = bundle_root.ok_or_else(|| "missing --bundle-root".to_string())?;

    let manifest: CandidateManifest = read_json(Path::new(&manifest_path))?;
    let ledger: EvaluatorLedger = read_json(Path::new(&ledger_path))?;
    let predictions: PredictionFile = read_json(Path::new(&predictions_path))?;
    let score = validate_and_score(&manifest, &ledger, &predictions, Path::new(&bundle_root))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&score)
            .map_err(|error| format!("cannot serialize score: {error}"))?
    );
    if score.unsafe_accepts != expected_unsafe {
        return Err(format!(
            "unsafe_accepts mismatch: expected {}, observed {}",
            expected_unsafe, score.unsafe_accepts
        ));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cfi-space-bench: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    const MANIFEST: &str = include_str!(
        "../../../research/spacecraft/cfi-space/fixtures/synthetic-candidate-manifest-v0.1.json"
    );
    const LEDGER: &str = include_str!(
        "../../../research/spacecraft/cfi-space/fixtures/synthetic-evaluator-ledger-v0.2.json"
    );
    const SAFE_PREDICTIONS: &str = include_str!(
        "../../../research/spacecraft/cfi-space/fixtures/synthetic-pose-predictions-safe-v0.2.json"
    );
    const UNSAFE_PREDICTIONS: &str = include_str!(
        "../../../research/spacecraft/cfi-space/fixtures/synthetic-pose-predictions-unsafe-v0.2.json"
    );

    fn parse<T: serde::de::DeserializeOwned>(source: &str) -> T {
        serde_json::from_str(source).unwrap()
    }

    #[test]
    fn known_answer_safe_fixture_has_no_unsafe_accepts() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);
        let score =
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).unwrap();

        assert_eq!(score.episodes, 10);
        assert_eq!(score.independent_units, 2);
        assert_eq!(score.counterfactual_groups, 2);
        assert_eq!(score.clean_episodes, 2);
        assert_eq!(score.fault_episodes, 8);
        assert_eq!(score.unsafe_accepts, 0);
        assert_eq!(score.false_abstentions, 0);
        assert_eq!(score.correctly_contained_faults, 8);
        assert_eq!(score.exact_disposition_matches, 10);
        assert_eq!(score.pose_estimates, 2);
        assert_eq!(score.pose_region_covered, 2);
        assert_eq!(score.pose_silent_misses, 0);
        assert_eq!(score.pose_summary.as_ref().unwrap().silent_misses, 0);
        assert_eq!(score.pose_metrics.len(), 2);
        assert_eq!(score.generalization.len(), 2);
        assert_eq!(
            score.generalization[0].stratum,
            GeneralizationStratum::SeenTarget
        );
        assert_eq!(score.generalization[0].episodes, 8);
        assert_eq!(score.generalization[0].fault_episodes, 7);
        assert_eq!(score.generalization[0].critical_fault_episodes, 6);
        assert_eq!(
            score.generalization[1].stratum,
            GeneralizationStratum::UnseenTarget
        );
        assert_eq!(score.generalization[1].episodes, 2);
        assert_eq!(score.generalization[1].fault_episodes, 1);
    }

    #[test]
    fn unsafe_common_cause_acceptance_is_counted() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(UNSAFE_PREDICTIONS);
        let score =
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).unwrap();

        assert_eq!(score.unsafe_accepts, 1);
        assert_eq!(score.false_abstentions, 0);
        assert_eq!(score.exact_disposition_matches, 9);
        assert_eq!(score.pose_estimates, 3);
        assert_eq!(score.pose_region_covered, 2);
        assert_eq!(score.pose_silent_misses, 1);
        assert_eq!(score.pose_summary.as_ref().unwrap().silent_misses, 1);
        let common = score.pose_metrics.iter().find(|row| row.episode_id == "episode-a-common").unwrap();
        assert!((common.translation_error_m - 0.5).abs() < 1e-9);
        assert!((common.rotation_error_deg - 45.0).abs() < 1e-6);
    }

    #[test]
    fn frame_sequence_gap_is_rejected() {
        let mut manifest: CandidateManifest = parse(MANIFEST);
        manifest.episodes[0].frames[1].sequence = 2;
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn counterfactual_split_leakage_is_rejected() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let mut ledger: EvaluatorLedger = parse(LEDGER);
        ledger.episodes[1].split = Split::Calibration;
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn counterfactual_group_requires_exactly_one_clean_reference() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let mut ledger: EvaluatorLedger = parse(LEDGER);
        ledger
            .episodes
            .retain(|episode| episode.episode_id != "episode-a-clean");
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn prediction_digest_substitution_is_rejected() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let mut predictions: PredictionFile = parse(SAFE_PREDICTIONS);
        predictions.predictions[0].input_bundle_sha256 = "f".repeat(64);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn source_bundle_frame_identity_mismatch_is_rejected() {
        let mut manifest: CandidateManifest = parse(MANIFEST);
        manifest.episodes[0].frames[0].frame_id = "substituted-frame".to_string();
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn source_bundle_horizon_metadata_mismatch_is_rejected() {
        let mut manifest: CandidateManifest = parse(MANIFEST);
        manifest.episodes[0].horizon_us += 1;
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn input_bundle_digest_substitution_is_rejected() {
        let mut manifest: CandidateManifest = parse(MANIFEST);
        manifest.episodes[0].input_bundle_sha256 = "f".repeat(64);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn input_bundle_path_traversal_is_rejected() {
        let mut manifest: CandidateManifest = parse(MANIFEST);
        manifest.episodes[0].input_bundle_path = "../Cargo.toml".to_string();
        let ledger: EvaluatorLedger = parse(LEDGER);
        let predictions: PredictionFile = parse(SAFE_PREDICTIONS);

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }

    #[test]
    fn duplicate_prediction_identity_is_rejected() {
        let manifest: CandidateManifest = parse(MANIFEST);
        let ledger: EvaluatorLedger = parse(LEDGER);
        let mut predictions: PredictionFile = parse(SAFE_PREDICTIONS);
        predictions.predictions[1].episode_id = predictions.predictions[0].episode_id.clone();

        assert!(
            validate_and_score(&manifest, &ledger, &predictions, Path::new(REPO_ROOT)).is_err()
        );
    }
}
