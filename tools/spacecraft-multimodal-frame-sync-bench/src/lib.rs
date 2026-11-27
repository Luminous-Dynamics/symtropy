#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-multimodal-frame-sync.receipt.v0.1";
pub const SPACESENSE_REVISION: &str = "f8c4ee9c7c0f9a4037f4977f3ddc4e0610a8fb8e";
pub const SPACESENSE_POSE_CORRECTION_DATE: &str = "2026-05-19";
pub const SPACESENSE_POSE_CSV_RELEASE: &str = "pose-ground-truth-regenerated-2026-05-19";
pub const TIMESTAMP_FORMAT: &str = "YYYYMMDDHHMMSSmmm";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleDisposition {
    Synchronized,
    Incomplete,
    Unsynchronized,
    OutOfScope,
    InvalidInput,
    DuplicateSample,
    SplitLeakage,
}

impl BundleDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Synchronized => "SYNCHRONIZED",
            Self::Incomplete => "INCOMPLETE",
            Self::Unsynchronized => "UNSYNCHRONIZED",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::InvalidInput => "INVALID_INPUT",
            Self::DuplicateSample => "DUPLICATE_SAMPLE",
            Self::SplitLeakage => "SPLIT_LEAKAGE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalityRecord {
    pub timestamp: String,
    pub frame_id: String,
    pub sensor_id: String,
    pub path: String,
    pub sha256: String,
    pub present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInput {
    pub case_id: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub license: String,
    pub intended_use: String,
    pub spacecraft_id: String,
    pub trajectory_id: String,
    pub sample_id: String,
    pub frame_id: String,
    pub split: String,
    pub timestamp_format: String,
    pub capture_timestamp: String,
    pub corrected_pose_revision: String,
    pub pose_csv_revision: String,
    pub camera_frame: String,
    pub world_frame: String,
    pub lidar_coordinate_frame: String,
    pub pose_semantics: String,
    pub quaternion_convention: String,
    pub camera_calibration_revision: String,
    pub camera_calibration_sha256: String,
    pub lidar_to_camera_revision: String,
    pub lidar_to_camera_sha256: String,
    pub pose_length_unit: String,
    pub depth_unit: String,
    pub lidar_unit: String,
    pub rgb: ModalityRecord,
    pub depth: ModalityRecord,
    pub segmentation: ModalityRecord,
    pub lidar: ModalityRecord,
    pub pose: ModalityRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleResult {
    pub case_id: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub license: String,
    pub spacecraft_id: String,
    pub trajectory_id: String,
    pub sample_id: String,
    pub frame_id: String,
    pub split: String,
    pub capture_timestamp: String,
    pub pose_csv_revision: String,
    pub corrected_pose_revision: String,
    pub camera_calibration_revision: String,
    pub camera_calibration_sha256: String,
    pub lidar_to_camera_revision: String,
    pub lidar_to_camera_sha256: String,
    pub lidar_coordinate_frame: String,
    pub pose_semantics: String,
    pub disposition: BundleDisposition,
    pub modality_sha256: BTreeMap<String, String>,
    pub modality_paths: BTreeMap<String, String>,
    pub reason_codes: Vec<String>,
    pub bundle_admissible: bool,
    pub source_bytes_authenticated: bool,
    pub calibration_physically_validated: bool,
    pub authority_granted: bool,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn divisible(value: u32, divisor: u32) -> bool {
    value
        .checked_div(divisor)
        .and_then(|quotient| quotient.checked_mul(divisor))
        == Some(value)
}

fn leap_year(year: u32) -> bool {
    divisible(year, 4) && (!divisible(year, 100) || divisible(year, 400))
}

fn valid_timestamp(value: &str) -> bool {
    if value.len() != 17 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let parse = |start: usize, end: usize| value[start..end].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second), Some(millis)) = (
        parse(0, 4),
        parse(4, 6),
        parse(6, 8),
        parse(8, 10),
        parse(10, 12),
        parse(12, 14),
        parse(14, 17),
    ) else {
        return false;
    };

    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };

    year > 0
        && day >= 1
        && day <= days_in_month
        && hour <= 23
        && minute <= 59
        && second <= 59
        && millis <= 999
}

fn modalities(input: &BundleInput) -> [(&'static str, &ModalityRecord, &'static str); 5] {
    [
        ("rgb", &input.rgb, "cam0"),
        ("depth", &input.depth, "cam0"),
        ("segmentation", &input.segmentation, "cam0"),
        ("lidar", &input.lidar, "lidar0"),
        ("pose", &input.pose, "pose_ground_truth"),
    ]
}

fn result(input: &BundleInput, disposition: BundleDisposition, reason: &str) -> BundleResult {
    let mut modality_sha256 = BTreeMap::new();
    let mut modality_paths = BTreeMap::new();
    for (name, record, _) in modalities(input) {
        modality_sha256.insert(name.to_owned(), record.sha256.clone());
        modality_paths.insert(name.to_owned(), record.path.clone());
    }

    BundleResult {
        case_id: input.case_id.clone(),
        dataset_id: input.dataset_id.clone(),
        dataset_revision: input.dataset_revision.clone(),
        license: input.license.clone(),
        spacecraft_id: input.spacecraft_id.clone(),
        trajectory_id: input.trajectory_id.clone(),
        sample_id: input.sample_id.clone(),
        frame_id: input.frame_id.clone(),
        split: input.split.clone(),
        capture_timestamp: input.capture_timestamp.clone(),
        pose_csv_revision: input.pose_csv_revision.clone(),
        corrected_pose_revision: input.corrected_pose_revision.clone(),
        camera_calibration_revision: input.camera_calibration_revision.clone(),
        camera_calibration_sha256: input.camera_calibration_sha256.clone(),
        lidar_to_camera_revision: input.lidar_to_camera_revision.clone(),
        lidar_to_camera_sha256: input.lidar_to_camera_sha256.clone(),
        lidar_coordinate_frame: input.lidar_coordinate_frame.clone(),
        pose_semantics: input.pose_semantics.clone(),
        disposition,
        modality_sha256,
        modality_paths,
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        bundle_admissible: disposition == BundleDisposition::Synchronized,
        source_bytes_authenticated: false,
        calibration_physically_validated: false,
        authority_granted: false,
    }
}

/// Verifies metadata-level identity and timestamp coherence. Timestamp strings
/// are validated and compared lexically in the source-declared calendar format;
/// no timezone, UTC offset or TAI epoch is inferred.
pub fn evaluate(input: &BundleInput) -> BundleResult {
    if [
        input.case_id.as_str(),
        input.dataset_id.as_str(),
        input.dataset_revision.as_str(),
        input.license.as_str(),
        input.intended_use.as_str(),
        input.spacecraft_id.as_str(),
        input.trajectory_id.as_str(),
        input.sample_id.as_str(),
        input.frame_id.as_str(),
        input.capture_timestamp.as_str(),
        input.corrected_pose_revision.as_str(),
        input.pose_csv_revision.as_str(),
    ]
    .iter()
    .any(|value| !valid_text(value))
    {
        return result(
            input,
            BundleDisposition::InvalidInput,
            "BUNDLE_IDENTITY_MISSING",
        );
    }

    if !matches!(input.split.as_str(), "train" | "validation" | "test") {
        return result(input, BundleDisposition::InvalidInput, "UNKNOWN_SPLIT");
    }

    if input.dataset_id != "SpaceSense-Bench"
        || input.dataset_revision != SPACESENSE_REVISION
        || input.license != "CC-BY-NC-4.0"
        || input.intended_use != "NONCOMMERCIAL"
        || input.corrected_pose_revision != SPACESENSE_POSE_CSV_RELEASE
        || input.camera_frame != "FRD_X_FORWARD_Y_RIGHT_Z_DOWN"
        || input.world_frame != "AIRSIM_NED"
        || input.lidar_coordinate_frame != "SENSOR_LOCAL_FRAME"
        || input.pose_semantics != "TARGET_IN_CAMERA_AND_CAMERA_POSE_IN_WORLD"
        || input.quaternion_convention != "HAMILTON_WXYZ"
        || !valid_sha256(&input.camera_calibration_sha256)
        || !valid_sha256(&input.lidar_to_camera_sha256)
        || input.pose_length_unit != "metre"
        || input.depth_unit != "millimetre"
        || input.lidar_unit != "metre"
        || input.timestamp_format != TIMESTAMP_FORMAT
    {
        return result(
            input,
            BundleDisposition::OutOfScope,
            "DATASET_REVISION_LICENSE_OR_CONVENTION_UNSUPPORTED",
        );
    }

    if input.pose_csv_revision != input.corrected_pose_revision {
        return result(
            input,
            BundleDisposition::OutOfScope,
            "POSE_CSV_NOT_FROM_CORRECTED_FRAME_ALIGNED_REVISION",
        );
    }

    if input.sample_id
        != format!(
            "{}/{}/{}",
            input.spacecraft_id, input.trajectory_id, input.capture_timestamp
        )
    {
        return result(
            input,
            BundleDisposition::OutOfScope,
            "SAMPLE_ID_DOES_NOT_BIND_SPACECRAFT_TRAJECTORY_AND_TIMESTAMP",
        );
    }

    if !valid_timestamp(&input.capture_timestamp) {
        return result(
            input,
            BundleDisposition::InvalidInput,
            "CAPTURE_TIMESTAMP_INVALID",
        );
    }

    for (name, record, expected_sensor) in modalities(input) {
        if !record.present {
            return result(
                input,
                BundleDisposition::Incomplete,
                match name {
                    "rgb" => "RGB_MODALITY_MISSING",
                    "depth" => "DEPTH_MODALITY_MISSING",
                    "segmentation" => "SEGMENTATION_MODALITY_MISSING",
                    "lidar" => "LIDAR_MODALITY_MISSING",
                    _ => "POSE_MODALITY_MISSING",
                },
            );
        }
        if !valid_text(&record.path) || !valid_sha256(&record.sha256) {
            return result(
                input,
                BundleDisposition::InvalidInput,
                "MODALITY_PATH_OR_DIGEST_INVALID",
            );
        }
        let required_path_scope = format!("{}/{}/", input.spacecraft_id, input.trajectory_id);
        if !record.path.contains(&required_path_scope)
            || !record.path.contains(&input.capture_timestamp)
        {
            return result(
                input,
                BundleDisposition::OutOfScope,
                "MODALITY_PATH_DOES_NOT_BIND_TRAJECTORY_AND_FRAME_TIME",
            );
        }
        if !valid_text(&record.sensor_id)
            || record.sensor_id != expected_sensor
            || record.frame_id != input.frame_id
        {
            return result(
                input,
                BundleDisposition::OutOfScope,
                "MODALITY_FRAME_OR_SENSOR_ID_MISMATCH",
            );
        }
        if !valid_timestamp(&record.timestamp) {
            return result(
                input,
                BundleDisposition::InvalidInput,
                "MODALITY_TIMESTAMP_INVALID",
            );
        }
        if record.timestamp != input.capture_timestamp {
            return result(
                input,
                BundleDisposition::Unsynchronized,
                "MODALITY_TIMESTAMP_DIFFERS_FROM_CAPTURE_EVENT",
            );
        }
    }

    result(input, BundleDisposition::Synchronized, "")
}

/// Detects duplicate sample keys and spacecraft leakage across splits. Results
/// remain aligned with the input records so every rejected row is retained.
pub fn evaluate_manifest(inputs: &[BundleInput]) -> Vec<BundleResult> {
    let mut spacecraft_splits: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for input in inputs {
        spacecraft_splits
            .entry(input.spacecraft_id.as_str())
            .or_default()
            .insert(input.split.as_str());
    }

    let mut duplicate_samples = BTreeSet::new();
    let mut seen_samples = BTreeSet::new();
    for input in inputs {
        if !seen_samples.insert(input.sample_id.as_str()) {
            duplicate_samples.insert(input.sample_id.as_str());
        }
    }

    let split_leaks: BTreeSet<&str> = spacecraft_splits
        .iter()
        .filter_map(|(spacecraft, splits)| {
            (splits.len() > 1).then_some(*spacecraft)
        })
        .collect();

    inputs
        .iter()
        .map(|input| {
            if split_leaks.contains(input.spacecraft_id.as_str()) {
                result(
                    input,
                    BundleDisposition::SplitLeakage,
                    "SPACECRAFT_ID_PRESENT_IN_MULTIPLE_SPLITS",
                )
            } else if duplicate_samples.contains(input.sample_id.as_str()) {
                result(
                    input,
                    BundleDisposition::DuplicateSample,
                    "SAMPLE_ID_DUPLICATED",
                )
            } else {
                evaluate(input)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        BundleDisposition, BundleInput, ModalityRecord, SPACESENSE_POSE_CSV_RELEASE,
        SPACESENSE_REVISION, evaluate, evaluate_manifest,
    };

    fn modality(name: &str, sensor: &str) -> ModalityRecord {
        ModalityRecord {
            timestamp: "20260120165126242".to_owned(),
            frame_id: "frame-0001".to_owned(),
            sensor_id: sensor.to_owned(),
            path: format!("raw/satellite/approach_front/{name}/frame-0001"),
            sha256: "a".repeat(64),
            present: true,
        }
    }

    fn input() -> BundleInput {
        BundleInput {
            case_id: "sync-1".to_owned(),
            dataset_id: "SpaceSense-Bench".to_owned(),
            dataset_revision: SPACESENSE_REVISION.to_owned(),
            license: "CC-BY-NC-4.0".to_owned(),
            intended_use: "NONCOMMERCIAL".to_owned(),
            spacecraft_id: "ACE".to_owned(),
            trajectory_id: "approach_front".to_owned(),
            sample_id: "ACE/approach_front/20260120165126242".to_owned(),
            frame_id: "frame-0001".to_owned(),
            split: "train".to_owned(),
            timestamp_format: "YYYYMMDDHHMMSSmmm".to_owned(),
            capture_timestamp: "20260120165126242".to_owned(),
            corrected_pose_revision: SPACESENSE_POSE_CSV_RELEASE.to_owned(),
            pose_csv_revision: SPACESENSE_POSE_CSV_RELEASE.to_owned(),
            camera_frame: "FRD_X_FORWARD_Y_RIGHT_Z_DOWN".to_owned(),
            world_frame: "AIRSIM_NED".to_owned(),
            lidar_coordinate_frame: "SENSOR_LOCAL_FRAME".to_owned(),
            pose_semantics: "TARGET_IN_CAMERA_AND_CAMERA_POSE_IN_WORLD".to_owned(),
            quaternion_convention: "HAMILTON_WXYZ".to_owned(),
            camera_calibration_revision: "cam0-cal-r1".to_owned(),
            camera_calibration_sha256: "b".repeat(64),
            lidar_to_camera_revision: "lidar0-to-cam0-r1".to_owned(),
            lidar_to_camera_sha256: "c".repeat(64),
            pose_length_unit: "metre".to_owned(),
            depth_unit: "millimetre".to_owned(),
            lidar_unit: "metre".to_owned(),
            rgb: modality("image", "cam0"),
            depth: modality("depth", "cam0"),
            segmentation: modality("seg", "cam0"),
            lidar: modality("lidar", "lidar0"),
            pose: modality("pose_ground_truth.csv", "pose_ground_truth"),
        }
    }

    #[test]
    fn exact_synchronized_bundle_is_admissible_but_not_authenticated() {
        let result = evaluate(&input());
        assert_eq!(result.disposition, BundleDisposition::Synchronized);
        assert!(result.bundle_admissible);
        assert!(!result.source_bytes_authenticated);
        assert!(!result.calibration_physically_validated);
        assert!(!result.authority_granted);
    }

    #[test]
    fn one_millisecond_pose_shift_is_unsynchronized() {
        let mut value = input();
        value.pose.timestamp = "20260120165126342".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            BundleDisposition::Unsynchronized
        );
    }

    #[test]
    fn rgb_depth_and_segmentation_must_match_capture_timestamp() {
        for field in ["rgb", "depth", "segmentation"] {
            let mut value = input();
            match field {
                "rgb" => value.rgb.timestamp = "20260120165126241".to_owned(),
                "depth" => value.depth.timestamp = "20260120165126243".to_owned(),
                _ => value.segmentation.timestamp = "20260120165126244".to_owned(),
            }
            assert_eq!(
                evaluate(&value).disposition,
                BundleDisposition::Unsynchronized
            );
        }
    }

    #[test]
    fn lidar_timestamp_is_exact_in_v0_1() {
        let mut value = input();
        value.lidar.timestamp = "20260120165126243".to_owned();
        assert_eq!(evaluate(&value).disposition, BundleDisposition::Unsynchronized);
    }

    #[test]
    fn stale_pre_correction_pose_csv_is_out_of_scope() {
        let mut value = input();
        value.pose_csv_revision = "initial-before-2026-05-19-fix".to_owned();
        assert_eq!(evaluate(&value).disposition, BundleDisposition::OutOfScope);
    }

    #[test]
    fn wrong_modality_frame_or_sensor_is_out_of_scope() {
        let mut wrong_frame = input();
        wrong_frame.lidar.frame_id = "another-frame".to_owned();
        assert_eq!(
            evaluate(&wrong_frame).disposition,
            BundleDisposition::OutOfScope
        );

        let mut wrong_sensor = input();
        wrong_sensor.depth.sensor_id = "cam1".to_owned();
        assert_eq!(
            evaluate(&wrong_sensor).disposition,
            BundleDisposition::OutOfScope
        );
    }

    #[test]
    fn wrong_trajectory_path_is_out_of_scope() {
        let mut value = input();
        value.pose.path =
            "raw/another-spacecraft/other-trajectory/pose/20260120165126242".to_owned();
        assert_eq!(evaluate(&value).disposition, BundleDisposition::OutOfScope);
    }

    #[test]
    fn sample_id_must_bind_exact_frame_key() {
        let mut value = input();
        value.sample_id = "ACE/approach_front/another-timestamp".to_owned();
        assert_eq!(evaluate(&value).disposition, BundleDisposition::OutOfScope);
    }

    #[test]
    fn missing_modality_is_incomplete_not_silently_dropped() {
        let mut value = input();
        value.lidar.present = false;
        assert_eq!(evaluate(&value).disposition, BundleDisposition::Incomplete);
    }

    #[test]
    fn timestamp_calendar_and_format_are_validated() {
        let mut malformed = input();
        malformed.capture_timestamp = "20260230000000000".to_owned();
        assert_eq!(
            evaluate(&malformed).disposition,
            BundleDisposition::InvalidInput
        );

        let mut malformed_format = input();
        malformed_format.rgb.timestamp = "2026-01-20T16:51:26.242".to_owned();
        assert_eq!(
            evaluate(&malformed_format).disposition,
            BundleDisposition::InvalidInput
        );
    }

    #[test]
    fn unknown_timezone_is_not_inferred_or_needed_for_exact_key_match() {
        let mut value = input();
        value.capture_timestamp = "20260120165126243".to_owned();
        value.sample_id = "ACE/approach_front/20260120165126243".to_owned();
        for record in [
            &mut value.rgb,
            &mut value.depth,
            &mut value.segmentation,
            &mut value.lidar,
            &mut value.pose,
        ] {
            record.timestamp = value.capture_timestamp.clone();
        }
        assert_eq!(
            evaluate(&value).disposition,
            BundleDisposition::Synchronized
        );
    }

    #[test]
    fn lidar_frame_and_pose_semantics_are_explicit() {
        let mut wrong_lidar_frame = input();
        wrong_lidar_frame.lidar_coordinate_frame = "CAMERA_FRD".to_owned();
        assert_eq!(
            evaluate(&wrong_lidar_frame).disposition,
            BundleDisposition::OutOfScope
        );

        let mut wrong_pose_semantics = input();
        wrong_pose_semantics.pose_semantics = "CAMERA_TO_WORLD_ONLY".to_owned();
        assert_eq!(
            evaluate(&wrong_pose_semantics).disposition,
            BundleDisposition::OutOfScope
        );
    }

    #[test]
    fn malformed_calibration_or_extrinsics_digest_is_out_of_scope() {
        let mut camera = input();
        camera.camera_calibration_sha256 = "bad".to_owned();
        assert_eq!(evaluate(&camera).disposition, BundleDisposition::OutOfScope);

        let mut extrinsics = input();
        extrinsics.lidar_to_camera_sha256 = "bad".to_owned();
        assert_eq!(
            evaluate(&extrinsics).disposition,
            BundleDisposition::OutOfScope
        );
    }

    #[test]
    fn wrong_license_or_commercial_intent_is_out_of_scope() {
        let mut value = input();
        value.intended_use = "COMMERCIAL".to_owned();
        assert_eq!(evaluate(&value).disposition, BundleDisposition::OutOfScope);
    }

    #[test]
    fn spacecraft_split_leakage_blocks_all_rows_for_that_spacecraft() {
        let first = input();
        let mut second = input();
        second.case_id = "sync-2".to_owned();
        second.sample_id = "ACE/test/other-frame".to_owned();
        second.frame_id = "frame-0002".to_owned();
        second.split = "test".to_owned();
        second.capture_timestamp = "20260120165127242".to_owned();
        for record in [
            &mut second.rgb,
            &mut second.depth,
            &mut second.segmentation,
            &mut second.lidar,
            &mut second.pose,
        ] {
            record.timestamp = second.capture_timestamp.clone();
            record.frame_id = "frame-0002".to_owned();
        }
        let results = evaluate_manifest(&[first, second]);
        assert!(
            results
                .iter()
                .all(|result| result.disposition == BundleDisposition::SplitLeakage)
        );
    }

    #[test]
    fn duplicate_sample_keys_are_rejected() {
        let first = input();
        let mut second = input();
        second.case_id = "sync-2".to_owned();
        let results = evaluate_manifest(&[first, second]);
        assert!(
            results
                .iter()
                .all(|result| result.disposition == BundleDisposition::DuplicateSample)
        );
    }
}
