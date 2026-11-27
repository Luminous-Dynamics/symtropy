#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-pose-region-scorecard.receipt.v0.1";
pub const NOMINAL_LEVELS: [u8; 5] = [50, 80, 90, 95, 99];
pub const QUATERNION_NORM_TOLERANCE: f64 = 1.0e-6;
pub const MAX_ANGLE_RADIUS_URAD: u64 = 3_141_593;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreDisposition {
    Scored,
    Abstained,
    Missing,
    InvalidInput,
    DuplicateSample,
    SplitLeakage,
}

impl ScoreDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scored => "SCORED",
            Self::Abstained => "ABSTAINED",
            Self::Missing => "MISSING",
            Self::InvalidInput => "INVALID_INPUT",
            Self::DuplicateSample => "DUPLICATE_SAMPLE",
            Self::SplitLeakage => "SPLIT_LEAKAGE",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoseRegion {
    pub lower_um: [i64; 3],
    pub upper_um: [i64; 3],
    pub orientation_radius_urad: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionInput {
    pub case_id: String,
    pub sample_id: String,
    pub target_id: String,
    pub split: String,
    pub crop_stratum: String,
    pub visible_parts: String,
    pub modality: String,
    pub ood_group: String,
    pub calibration_revision: String,
    pub frame_id: String,
    pub quaternion_order: String,
    pub translation_unit: String,
    pub pose_semantics: String,
    pub status: String,
    pub predicted_translation_um: [i64; 3],
    pub truth_translation_um: [i64; 3],
    pub predicted_quaternion: [f64; 4],
    pub truth_quaternion: [f64; 4],
    pub regions: [PoseRegion; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionScore {
    pub nominal_percent: u8,
    pub translation_axis_covered: [bool; 3],
    pub translation_joint_covered: bool,
    pub orientation_covered: bool,
    pub joint_covered: bool,
    pub orientation_error_urad: u64,
    pub width_um: [u64; 3],
    pub box_volume_um3: u128,
    pub orientation_radius_urad: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoseScore {
    pub case_id: String,
    pub sample_id: String,
    pub target_id: String,
    pub split: String,
    pub crop_stratum: String,
    pub visible_parts: String,
    pub modality: String,
    pub ood_group: String,
    pub calibration_revision: String,
    pub frame_id: String,
    pub disposition: ScoreDisposition,
    pub reason_codes: Vec<String>,
    pub regions: Vec<RegionScore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelAggregate {
    pub nominal_percent: u8,
    pub accepted_count: u64,
    pub translation_axis_covered: [u64; 3],
    pub translation_joint_covered: u64,
    pub orientation_covered: u64,
    pub joint_covered: u64,
    pub false_confident: u64,
    pub sum_width_um: [u128; 3],
    pub sum_box_volume_um3: u128,
    pub sum_orientation_radius_urad: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupScore {
    pub target_id: String,
    pub crop_stratum: String,
    pub modality: String,
    pub ood_group: String,
    pub sample_count: u64,
    pub accepted_count: u64,
    pub abstained_count: u64,
    pub missing_count: u64,
    pub invalid_count: u64,
    pub levels: Vec<LevelAggregate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scorecard {
    pub case_count: u64,
    pub scored_count: u64,
    pub abstained_count: u64,
    pub missing_count: u64,
    pub invalid_count: u64,
    pub duplicate_sample_count: u64,
    pub split_leakage_count: u64,
    pub levels: Vec<LevelAggregate>,
    pub groups: Vec<GroupScore>,
    pub results: Vec<PoseScore>,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn canonical_xyzw(order: &str, q: [f64; 4]) -> Option<[f64; 4]> {
    match order {
        "XYZW" => Some(q),
        "WXYZ" => Some([q[1], q[2], q[3], q[0]]),
        _ => None,
    }
}

fn normalized_quaternion(q: [f64; 4]) -> Option<[f64; 4]> {
    if !finite(&q) {
        return None;
    }
    let norm = q.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 || (norm - 1.0).abs() > QUATERNION_NORM_TOLERANCE {
        return None;
    }
    Some([q[0] / norm, q[1] / norm, q[2] / norm, q[3] / norm])
}

fn orientation_error_urad(predicted: [f64; 4], truth: [f64; 4]) -> Option<u64> {
    let dot = predicted
        .iter()
        .zip(truth.iter())
        .map(|(left, right)| left * right)
        .sum::<f64>()
        .abs()
        .clamp(0.0, 1.0);
    let angle = 2.0 * dot.acos();
    let scaled = (angle * 1_000_000.0).ceil();
    if !scaled.is_finite() || scaled < 0.0 || scaled > MAX_ANGLE_RADIUS_URAD as f64 {
        return None;
    }
    Some(scaled as u64)
}

fn base_score(input: &PredictionInput, disposition: ScoreDisposition, reason: &str) -> PoseScore {
    PoseScore {
        case_id: input.case_id.clone(),
        sample_id: input.sample_id.clone(),
        target_id: input.target_id.clone(),
        split: input.split.clone(),
        crop_stratum: input.crop_stratum.clone(),
        visible_parts: input.visible_parts.clone(),
        modality: input.modality.clone(),
        ood_group: input.ood_group.clone(),
        calibration_revision: input.calibration_revision.clone(),
        frame_id: input.frame_id.clone(),
        disposition,
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        regions: Vec::new(),
    }
}

fn validate_identity(input: &PredictionInput) -> Option<&'static str> {
    if [
        input.case_id.as_str(),
        input.sample_id.as_str(),
        input.target_id.as_str(),
        input.split.as_str(),
        input.crop_stratum.as_str(),
        input.visible_parts.as_str(),
        input.modality.as_str(),
        input.ood_group.as_str(),
        input.calibration_revision.as_str(),
        input.frame_id.as_str(),
        input.pose_semantics.as_str(),
    ]
    .iter()
    .any(|value| !valid_text(value))
    {
        return Some("IDENTITY_OR_GROUP_METADATA_MISSING");
    }
    if !matches!(input.split.as_str(), "train" | "validation" | "test") {
        return Some("UNKNOWN_SPLIT");
    }
    if input.translation_unit != "micrometre" {
        return Some("UNSUPPORTED_TRANSLATION_UNIT");
    }
    if !matches!(input.quaternion_order.as_str(), "XYZW" | "WXYZ") {
        return Some("UNSUPPORTED_QUATERNION_ORDER");
    }
    None
}

fn score_accepted(input: &PredictionInput) -> Result<Vec<RegionScore>, &'static str> {
    if let Some(reason) = validate_identity(input) {
        return Err(reason);
    }

    let predicted_q = canonical_xyzw(input.quaternion_order.as_str(), input.predicted_quaternion)
        .and_then(normalized_quaternion)
        .ok_or("PREDICTED_QUATERNION_INVALID")?;
    let truth_q = canonical_xyzw(input.quaternion_order.as_str(), input.truth_quaternion)
        .and_then(normalized_quaternion)
        .ok_or("TRUTH_QUATERNION_INVALID")?;
    let orientation_error = orientation_error_urad(predicted_q, truth_q)
        .ok_or("ORIENTATION_ERROR_UNREPRESENTABLE")?;

    let mut scores = Vec::with_capacity(5);
    let mut previous: Option<&PoseRegion> = None;

    for (index, region) in input.regions.iter().enumerate() {
        if region.orientation_radius_urad > MAX_ANGLE_RADIUS_URAD {
            return Err("ORIENTATION_REGION_RADIUS_OUT_OF_RANGE");
        }

        let mut width = [0_u64; 3];
        let mut axis_covered = [false; 3];
        for (axis, ((&lower, &upper), (&predicted, &truth))) in region
            .lower_um
            .iter()
            .zip(region.upper_um.iter())
            .zip(
                input
                    .predicted_translation_um
                    .iter()
                    .zip(input.truth_translation_um.iter()),
            )
            .enumerate()
        {
            if lower > upper {
                return Err("TRANSLATION_REGION_BOUNDS_REVERSED");
            }
            if predicted < lower || predicted > upper {
                return Err("PREDICTION_CENTER_OUTSIDE_DECLARED_REGION");
            }
            let span = i128::from(upper)
                .checked_sub(i128::from(lower))
                .and_then(|value| u64::try_from(value).ok())
                .ok_or("TRANSLATION_REGION_WIDTH_OVERFLOW")?;
            width[axis] = span;
            axis_covered[axis] = truth >= lower && truth <= upper;
        }

        if let Some(inner) = previous {
            for ((&lower, &upper), (&inner_lower, &inner_upper)) in region
                .lower_um
                .iter()
                .zip(region.upper_um.iter())
                .zip(inner.lower_um.iter().zip(inner.upper_um.iter()))
            {
                if lower > inner_lower || upper < inner_upper {
                    return Err("POSE_REGIONS_NOT_NESTED");
                }
            }
            if region.orientation_radius_urad < inner.orientation_radius_urad {
                return Err("POSE_REGIONS_NOT_NESTED");
            }
        }

        let volume = u128::from(width[0])
            .checked_mul(u128::from(width[1]))
            .and_then(|value| value.checked_mul(u128::from(width[2])))
            .ok_or("TRANSLATION_REGION_VOLUME_OVERFLOW")?;
        let translation_joint = axis_covered.iter().all(|covered| *covered);
        let orientation_covered = orientation_error <= region.orientation_radius_urad;

        scores.push(RegionScore {
            nominal_percent: NOMINAL_LEVELS[index],
            translation_axis_covered: axis_covered,
            translation_joint_covered: translation_joint,
            orientation_covered,
            joint_covered: translation_joint && orientation_covered,
            orientation_error_urad: orientation_error,
            width_um: width,
            box_volume_um3: volume,
            orientation_radius_urad: region.orientation_radius_urad,
        });
        previous = Some(region);
    }

    Ok(scores)
}

pub fn evaluate(input: &PredictionInput) -> PoseScore {
    if let Some(reason) = validate_identity(input) {
        return base_score(input, ScoreDisposition::InvalidInput, reason);
    }

    match input.status.as_str() {
        "ABSTAINED" => return base_score(input, ScoreDisposition::Abstained, "PREDICTOR_ABSTAINED"),
        "MISSING" => return base_score(input, ScoreDisposition::Missing, "PREDICTION_MISSING"),
        "ACCEPTED" => {}
        _ => return base_score(input, ScoreDisposition::InvalidInput, "UNKNOWN_PREDICTION_STATUS"),
    }

    match score_accepted(input) {
        Ok(regions) => {
            let mut score = base_score(input, ScoreDisposition::Scored, "");
            score.regions = regions;
            score
        }
        Err(reason) => base_score(input, ScoreDisposition::InvalidInput, reason),
    }
}

fn empty_aggregate(level: u8) -> LevelAggregate {
    LevelAggregate {
        nominal_percent: level,
        accepted_count: 0,
        translation_axis_covered: [0; 3],
        translation_joint_covered: 0,
        orientation_covered: 0,
        joint_covered: 0,
        false_confident: 0,
        sum_width_um: [0; 3],
        sum_box_volume_um3: 0,
        sum_orientation_radius_urad: 0,
    }
}

fn aggregate(scores: &[&PoseScore]) -> Vec<LevelAggregate> {
    let mut levels = NOMINAL_LEVELS.map(empty_aggregate);
    for score in scores {
        if score.disposition != ScoreDisposition::Scored {
            continue;
        }
        for (index, region) in score.regions.iter().enumerate() {
            let entry = &mut levels[index];
            entry.accepted_count += 1;
            for ((covered_count, width_sum), (&covered, &width)) in entry
                .translation_axis_covered
                .iter_mut()
                .zip(entry.sum_width_um.iter_mut())
                .zip(
                    region
                        .translation_axis_covered
                        .iter()
                        .zip(region.width_um.iter()),
                )
            {
                if covered {
                    *covered_count += 1;
                }
                *width_sum += u128::from(width);
            }
            if region.translation_joint_covered {
                entry.translation_joint_covered += 1;
            }
            if region.orientation_covered {
                entry.orientation_covered += 1;
            }
            if region.joint_covered {
                entry.joint_covered += 1;
            } else {
                entry.false_confident += 1;
            }
            entry.sum_box_volume_um3 += region.box_volume_um3;
            entry.sum_orientation_radius_urad += u128::from(region.orientation_radius_urad);
        }
    }
    levels.to_vec()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    target_id: String,
    crop_stratum: String,
    modality: String,
    ood_group: String,
}

pub fn evaluate_manifest(inputs: &[PredictionInput]) -> Scorecard {
    let mut target_splits: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut seen_samples = BTreeSet::new();
    let mut duplicate_samples = BTreeSet::new();

    for input in inputs {
        target_splits.entry(&input.target_id).or_default().insert(&input.split);
        if !seen_samples.insert(input.sample_id.as_str()) {
            duplicate_samples.insert(input.sample_id.as_str());
        }
    }

    let leaked_targets: BTreeSet<&str> = target_splits
        .iter()
        .filter_map(|(target, splits)| (splits.len() > 1).then_some(*target))
        .collect();

    let mut results = Vec::with_capacity(inputs.len());
    for input in inputs {
        let score = if leaked_targets.contains(input.target_id.as_str()) {
            base_score(input, ScoreDisposition::SplitLeakage, "TARGET_ID_PRESENT_IN_MULTIPLE_SPLITS")
        } else if duplicate_samples.contains(input.sample_id.as_str()) {
            base_score(input, ScoreDisposition::DuplicateSample, "SAMPLE_ID_DUPLICATED")
        } else {
            evaluate(input)
        };
        results.push(score);
    }
    results.sort_by_key(|score| score.sample_id.clone());

    let all_refs: Vec<&PoseScore> = results.iter().collect();
    let levels = aggregate(&all_refs);
    let mut grouped: BTreeMap<GroupKey, Vec<&PoseScore>> = BTreeMap::new();
    for score in &results {
        grouped.entry(GroupKey {
            target_id: score.target_id.clone(),
            crop_stratum: score.crop_stratum.clone(),
            modality: score.modality.clone(),
            ood_group: score.ood_group.clone(),
        }).or_default().push(score);
    }

    let groups = grouped.into_iter().map(|(key, rows)| {
        GroupScore {
            target_id: key.target_id,
            crop_stratum: key.crop_stratum,
            modality: key.modality,
            ood_group: key.ood_group,
            sample_count: rows.len() as u64,
            accepted_count: rows.iter().filter(|row| row.disposition == ScoreDisposition::Scored).count() as u64,
            abstained_count: rows.iter().filter(|row| row.disposition == ScoreDisposition::Abstained).count() as u64,
            missing_count: rows.iter().filter(|row| row.disposition == ScoreDisposition::Missing).count() as u64,
            invalid_count: rows.iter().filter(|row| !matches!(row.disposition, ScoreDisposition::Scored | ScoreDisposition::Abstained | ScoreDisposition::Missing)).count() as u64,
            levels: aggregate(&rows),
        }
    }).collect();

    Scorecard {
        case_count: results.len() as u64,
        scored_count: results.iter().filter(|row| row.disposition == ScoreDisposition::Scored).count() as u64,
        abstained_count: results.iter().filter(|row| row.disposition == ScoreDisposition::Abstained).count() as u64,
        missing_count: results.iter().filter(|row| row.disposition == ScoreDisposition::Missing).count() as u64,
        invalid_count: results.iter().filter(|row| row.disposition == ScoreDisposition::InvalidInput).count() as u64,
        duplicate_sample_count: results.iter().filter(|row| row.disposition == ScoreDisposition::DuplicateSample).count() as u64,
        split_leakage_count: results.iter().filter(|row| row.disposition == ScoreDisposition::SplitLeakage).count() as u64,
        levels,
        groups,
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::{PoseRegion, PredictionInput, ScoreDisposition, evaluate, evaluate_manifest};

    fn regions() -> [PoseRegion; 5] {
        [
            PoseRegion { lower_um: [-1, -1, -1], upper_um: [1, 1, 1], orientation_radius_urad: 0 },
            PoseRegion { lower_um: [-2, -2, -2], upper_um: [2, 2, 2], orientation_radius_urad: 1 },
            PoseRegion { lower_um: [-3, -3, -3], upper_um: [3, 3, 3], orientation_radius_urad: 2 },
            PoseRegion { lower_um: [-4, -4, -4], upper_um: [4, 4, 4], orientation_radius_urad: 3 },
            PoseRegion { lower_um: [-5, -5, -5], upper_um: [5, 5, 5], orientation_radius_urad: 4 },
        ]
    }

    fn input(id: &str) -> PredictionInput {
        PredictionInput {
            case_id: id.to_owned(),
            sample_id: format!("sample-{id}"),
            target_id: "target-1".to_owned(),
            split: "test".to_owned(),
            crop_stratum: "100pct".to_owned(),
            visible_parts: "bus,panel,antenna".to_owned(),
            modality: "RGB".to_owned(),
            ood_group: "in_distribution".to_owned(),
            calibration_revision: "cal-r1".to_owned(),
            frame_id: "LVLH-r1".to_owned(),
            quaternion_order: "XYZW".to_owned(),
            translation_unit: "micrometre".to_owned(),
            pose_semantics: "TARGET_POSE_IN_DECLARED_FRAME".to_owned(),
            status: "ACCEPTED".to_owned(),
            predicted_translation_um: [0, 0, 0],
            truth_translation_um: [0, 0, 0],
            predicted_quaternion: [0.0, 0.0, 0.0, 1.0],
            truth_quaternion: [0.0, 0.0, 0.0, 1.0],
            regions: regions(),
        }
    }

    #[test]
    fn exact_pose_is_inside_all_five_nested_regions() {
        let result = evaluate(&input("exact"));
        assert_eq!(result.disposition, ScoreDisposition::Scored);
        assert!(result.regions.iter().all(|region| region.joint_covered));
    }

    #[test]
    fn quaternion_sign_flip_has_zero_geodesic_error() {
        let mut value = input("q-sign");
        value.truth_quaternion = [0.0, 0.0, 0.0, -1.0];
        let result = evaluate(&value);
        assert_eq!(result.regions[0].orientation_error_urad, 0);
        assert!(result.regions.iter().all(|region| region.orientation_covered));
    }

    #[test]
    fn translation_axis_and_orientation_coverage_are_separate() {
        let mut value = input("split-coverage");
        value.truth_translation_um = [2, 0, 0];
        value.truth_quaternion = [1.0, 0.0, 0.0, 0.0];
        let result = evaluate(&value);
        assert!(!result.regions[0].translation_axis_covered[0]);
        assert!(result.regions[4].translation_joint_covered);
        assert!(!result.regions[0].orientation_covered);
    }

    #[test]
    fn nonnested_pose_regions_are_invalid() {
        let mut value = input("nested");
        value.regions[4].lower_um[0] = 0;
        assert_eq!(evaluate(&value).disposition, ScoreDisposition::InvalidInput);
    }

    #[test]
    fn prediction_center_must_lie_inside_each_region() {
        let mut value = input("center");
        value.regions[0].upper_um[0] = -1;
        assert_eq!(evaluate(&value).disposition, ScoreDisposition::InvalidInput);
    }

    #[test]
    fn abstention_is_not_a_coverage_success() {
        let mut value = input("abstain");
        value.status = "ABSTAINED".to_owned();
        let result = evaluate(&value);
        assert_eq!(result.disposition, ScoreDisposition::Abstained);
        assert!(result.regions.is_empty());
    }

    #[test]
    fn missing_prediction_has_a_separate_disposition() {
        let mut value = input("missing");
        value.status = "MISSING".to_owned();
        assert_eq!(evaluate(&value).disposition, ScoreDisposition::Missing);
    }

    #[test]
    fn invalid_quaternion_never_receives_a_coverage_score() {
        let mut value = input("bad-q");
        value.predicted_quaternion = [0.0; 4];
        assert_eq!(evaluate(&value).disposition, ScoreDisposition::InvalidInput);
    }

    #[test]
    fn target_split_leakage_blocks_all_rows_for_target() {
        let first = input("train");
        let mut second = input("test");
        second.sample_id = "sample-test-2".to_owned();
        second.split = "train".to_owned();
        let result = evaluate_manifest(&[first, second]);
        assert!(result.iter().all(|row| row.disposition == ScoreDisposition::SplitLeakage));
    }

    #[test]
    fn duplicate_samples_are_not_counted_twice() {
        let first = input("dup");
        let mut second = first.clone();
        second.case_id = "dup2".to_owned();
        let result = evaluate_manifest(&[first, second]);
        assert!(result.iter().all(|row| row.disposition == ScoreDisposition::DuplicateSample));
    }
}
