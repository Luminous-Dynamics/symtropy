#![deny(unsafe_code)]

use std::collections::BTreeSet;

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-cross-modal-pose-consistency.receipt.v0.1";
pub const Q30_ONE: i64 = 1 << 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    Rgb,
    Event,
}

impl Modality {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "RGB" => Ok(Self::Rgb),
            "EVENT" => Ok(Self::Event),
            _ => Err(format!("unsupported modality: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Event => "EVENT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimateStatus {
    Candidate,
    Abstained,
    Invalid,
    OutOfDistribution,
}

impl EstimateStatus {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "CANDIDATE" => Ok(Self::Candidate),
            "ABSTAINED" => Ok(Self::Abstained),
            "INVALID" => Ok(Self::Invalid),
            "OUT_OF_DISTRIBUTION" => Ok(Self::OutOfDistribution),
            _ => Err(format!("unsupported estimate status: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "CANDIDATE",
            Self::Abstained => "ABSTAINED",
            Self::Invalid => "INVALID",
            Self::OutOfDistribution => "OUT_OF_DISTRIBUTION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyDigests {
    pub clock: String,
    pub optics: String,
    pub extrinsics: String,
    pub model: String,
    pub training_data: String,
    pub target_geometry: String,
    pub renderer: String,
    pub annotation: String,
    pub translator: String,
    pub declarations_complete: bool,
}

impl DependencyDigests {
    fn fields(&self) -> [(&'static str, &str); 9] {
        [
            ("clock", &self.clock),
            ("optics", &self.optics),
            ("extrinsics", &self.extrinsics),
            ("model", &self.model),
            ("training_data", &self.training_data),
            ("target_geometry", &self.target_geometry),
            ("renderer", &self.renderer),
            ("annotation", &self.annotation),
            ("translator", &self.translator),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoseEstimate {
    pub modality: Modality,
    pub sample_id: String,
    pub frame_id: String,
    pub target_id: String,
    pub timestamp_ms: i64,
    pub frame_convention: String,
    pub transform_direction: String,
    pub translation_unit: String,
    pub quaternion_order: String,
    pub translation_um: [i64; 3],
    /// Unit quaternion in XYZW order, signed Q30 integers. q and -q are equivalent.
    pub quaternion_xyzw_q30: [i64; 4],
    pub status: EstimateStatus,
    pub ood: bool,
    pub low_observability: bool,
    pub dependencies: DependencyDigests,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatePolicy {
    pub maximum_timestamp_skew_ms: u64,
    pub maximum_translation_disagreement_um: u64,
    /// Q30 Euclidean chord distance on the unit-quaternion double cover.
    /// For unit quaternions, chord = 2 sin(theta/4), where theta is SO(3) geodesic angle.
    pub maximum_orientation_chord_q30: u64,
    pub require_complete_dependency_declarations: bool,
    pub shared_dependencies_fail_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyReceipt {
    pub schema: String,
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub sample_id: String,
    pub frame_id: String,
    pub target_id: String,
    pub timestamp_skew_ms: u64,
    pub translation_disagreement_um: Option<u64>,
    pub orientation_chord_q30: Option<u64>,
    pub shared_dependency_kinds: Vec<String>,
    pub rgb_estimate_preserved: bool,
    pub event_estimate_preserved: bool,
    pub independence_proven: bool,
    pub source_bytes_authenticated: bool,
    pub physical_truth_established: bool,
    pub deterministic_safety_bound: bool,
    pub maneuver_authority_granted: bool,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value != "NONE"
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn ceil_sqrt(value: u128) -> Option<u64> {
    if value == 0 {
        return Some(0);
    }
    let mut low = 0_u128;
    let mut high = 1_u128 << 64;
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle;
        }
    }
    let square = low.checked_mul(low)?;
    let root = if square == value { low } else { low.checked_add(1)? };
    u64::try_from(root).ok()
}

fn squared_norm_i64(values: &[i64]) -> Option<u128> {
    values.iter().try_fold(0_u128, |sum, value| {
        let magnitude = i128::from(*value).unsigned_abs();
        sum.checked_add(magnitude.checked_mul(magnitude)?)
    })
}

fn translation_disagreement(left: [i64; 3], right: [i64; 3]) -> Option<u64> {
    let delta = [
        left[0].checked_sub(right[0])?,
        left[1].checked_sub(right[1])?,
        left[2].checked_sub(right[2])?,
    ];
    ceil_sqrt(squared_norm_i64(&delta)?)
}

fn quaternion_is_unit(q: [i64; 4]) -> bool {
    let Some(norm_sq) = squared_norm_i64(&q) else {
        return false;
    };
    let one = u128::try_from(Q30_ONE).unwrap_or(0);
    let target = one * one;
    let tolerance = target / 1_000; // 0.1% squared-norm envelope
    norm_sq.abs_diff(target) <= tolerance
}

fn orientation_chord_q30(left: [i64; 4], right: [i64; 4]) -> Option<u64> {
    let mut direct = [0_i64; 4];
    let mut antipodal = [0_i64; 4];
    for index in 0..4 {
        direct[index] = left[index].checked_sub(right[index])?;
        antipodal[index] = left[index].checked_add(right[index])?;
    }
    let direct_norm = ceil_sqrt(squared_norm_i64(&direct)?)?;
    let antipodal_norm = ceil_sqrt(squared_norm_i64(&antipodal)?)?;
    Some(direct_norm.min(antipodal_norm))
}

fn validate_estimate(estimate: &PoseEstimate) -> Result<(), String> {
    for (name, value) in [
        ("sample_id", estimate.sample_id.as_str()),
        ("frame_id", estimate.frame_id.as_str()),
        ("target_id", estimate.target_id.as_str()),
        ("frame_convention", estimate.frame_convention.as_str()),
        ("transform_direction", estimate.transform_direction.as_str()),
        ("translation_unit", estimate.translation_unit.as_str()),
        ("quaternion_order", estimate.quaternion_order.as_str()),
    ] {
        if !valid_text(value) {
            return Err(format!("{name} is empty or reserved"));
        }
    }
    for (name, digest) in estimate.dependencies.fields() {
        if !valid_digest(digest) {
            return Err(format!("dependency digest {name} is not a SHA-256 hex string"));
        }
    }
    if !quaternion_is_unit(estimate.quaternion_xyzw_q30) {
        return Err("quaternion is not unit length within the declared Q30 tolerance".to_owned());
    }
    Ok(())
}

fn identity_compatible(rgb: &PoseEstimate, event: &PoseEstimate) -> bool {
    rgb.sample_id == event.sample_id
        && rgb.frame_id == event.frame_id
        && rgb.target_id == event.target_id
        && rgb.frame_convention == event.frame_convention
        && rgb.transform_direction == event.transform_direction
        && rgb.translation_unit == event.translation_unit
        && rgb.quaternion_order == event.quaternion_order
}

fn shared_dependencies(rgb: &PoseEstimate, event: &PoseEstimate) -> Vec<String> {
    rgb.dependencies
        .fields()
        .into_iter()
        .zip(event.dependencies.fields())
        .filter_map(|((name, left), (_, right))| {
            (left != "NONE" && left == right).then_some(name.to_owned())
        })
        .collect()
}

fn base_receipt(
    rgb: &PoseEstimate,
    event: &PoseEstimate,
    disposition: &str,
    reasons: Vec<String>,
    timestamp_skew_ms: u64,
) -> ConsistencyReceipt {
    ConsistencyReceipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        disposition: disposition.to_owned(),
        reason_codes: reasons,
        sample_id: rgb.sample_id.clone(),
        frame_id: rgb.frame_id.clone(),
        target_id: rgb.target_id.clone(),
        timestamp_skew_ms,
        translation_disagreement_um: None,
        orientation_chord_q30: None,
        shared_dependency_kinds: Vec::new(),
        rgb_estimate_preserved: true,
        event_estimate_preserved: true,
        independence_proven: false,
        source_bytes_authenticated: false,
        physical_truth_established: false,
        deterministic_safety_bound: false,
        maneuver_authority_granted: false,
    }
}

/// Fail-closed consistency gate for RGB/event pose estimates.
///
/// It does not fuse the estimates. Shared dependency digests are reported as possible
/// common causes; any shared dependency blocks a two-sensor consistency promotion when
/// policy.shared_dependencies_fail_closed is true. Different digests do not prove
/// statistical independence or absence of unrecorded common causes.
pub fn evaluate_pair(
    rgb: &PoseEstimate,
    event: &PoseEstimate,
    policy: &GatePolicy,
) -> Result<ConsistencyReceipt, String> {
    if rgb.modality != Modality::Rgb || event.modality != Modality::Event {
        return Err("pair must contain one RGB and one EVENT estimate".to_owned());
    }
    validate_estimate(rgb)?;
    validate_estimate(event)?;

    let timestamp_skew = i128::from(rgb.timestamp_ms)
        .abs_diff(i128::from(event.timestamp_ms));
    let timestamp_skew_ms = u64::try_from(timestamp_skew)
        .map_err(|_| "timestamp skew does not fit u64".to_owned())?;

    if !identity_compatible(rgb, event) {
        return Ok(base_receipt(
            rgb,
            event,
            "OUT_OF_SCOPE",
            vec!["FRAME_TARGET_OR_COORDINATE_SCOPE_MISMATCH".to_owned()],
            timestamp_skew_ms,
        ));
    }
    if timestamp_skew_ms > policy.maximum_timestamp_skew_ms {
        return Ok(base_receipt(
            rgb,
            event,
            "OUT_OF_SCOPE",
            vec!["TIMESTAMP_SKEW_EXCEEDS_POLICY".to_owned()],
            timestamp_skew_ms,
        ));
    }
    if rgb.status != EstimateStatus::Candidate || event.status != EstimateStatus::Candidate {
        let reason = if rgb.status == EstimateStatus::OutOfDistribution
            || event.status == EstimateStatus::OutOfDistribution
            || rgb.ood
            || event.ood
        {
            "MODALITY_OUT_OF_DISTRIBUTION"
        } else if rgb.status == EstimateStatus::Abstained || event.status == EstimateStatus::Abstained {
            "MODALITY_ABSTAINED"
        } else {
            "MODALITY_ESTIMATE_INVALID"
        };
        return Ok(base_receipt(
            rgb,
            event,
            "ABSTAIN_MODALITY_UNAVAILABLE",
            vec![reason.to_owned()],
            timestamp_skew_ms,
        ));
    }
    if rgb.low_observability || event.low_observability {
        return Ok(base_receipt(
            rgb,
            event,
            "ABSTAIN_LOW_OBSERVABILITY",
            vec!["LOW_OBSERVABILITY_IN_ONE_OR_MORE_MODALITIES".to_owned()],
            timestamp_skew_ms,
        ));
    }

    let translation = translation_disagreement(rgb.translation_um, event.translation_um)
        .ok_or_else(|| "translation disagreement overflow".to_owned())?;
    let orientation = orientation_chord_q30(
        rgb.quaternion_xyzw_q30,
        event.quaternion_xyzw_q30,
    )
    .ok_or_else(|| "orientation chord calculation overflow".to_owned())?;

    let mut receipt = base_receipt(
        rgb,
        event,
        "CANDIDATE_CONSISTENT",
        Vec::new(),
        timestamp_skew_ms,
    );
    receipt.translation_disagreement_um = Some(translation);
    receipt.orientation_chord_q30 = Some(orientation);

    if translation > policy.maximum_translation_disagreement_um
        || orientation > policy.maximum_orientation_chord_q30
    {
        receipt.disposition = "DISAGREEMENT".to_owned();
        receipt.reason_codes.push("POSE_DISAGREEMENT_EXCEEDS_POLICY".to_owned());
    }

    let mut shared = shared_dependencies(rgb, event);
    receipt.shared_dependency_kinds = shared.clone();
    if policy.require_complete_dependency_declarations
        && (!rgb.dependencies.declarations_complete || !event.dependencies.declarations_complete)
    {
        receipt.disposition = "DEPENDENCE_UNKNOWN".to_owned();
        receipt.reason_codes.push("DEPENDENCY_DECLARATIONS_INCOMPLETE".to_owned());
    }
    if policy.shared_dependencies_fail_closed && !shared.is_empty() {
        receipt.disposition = "DEPENDENCE_UNKNOWN".to_owned();
        receipt.reason_codes.push("SHARED_COMMON_CAUSE_DEPENDENCY".to_owned());
    }

    receipt.reason_codes.sort();
    receipt.reason_codes.dedup();
    shared.sort();
    receipt.shared_dependency_kinds = shared;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::{
        DependencyDigests, EstimateStatus, GatePolicy, Modality, PoseEstimate,
        evaluate_pair, Q30_ONE,
    };

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn estimate(modality: Modality, x_um: i64) -> PoseEstimate {
        let prefix = if modality == Modality::Rgb { 'a' } else { 'b' };
        PoseEstimate {
            modality,
            sample_id: "sample-001".to_owned(),
            frame_id: "frame-001".to_owned(),
            target_id: "TANGO".to_owned(),
            timestamp_ms: 1_000,
            frame_convention: "WORLD_RIGHT_HANDED".to_owned(),
            transform_direction: "TARGET_TO_WORLD".to_owned(),
            translation_unit: "metre".to_owned(),
            quaternion_order: "XYZW".to_owned(),
            translation_um: [x_um, 0, 0],
            quaternion_xyzw_q30: [0, 0, 0, Q30_ONE],
            status: EstimateStatus::Candidate,
            ood: false,
            low_observability: false,
            dependencies: DependencyDigests {
                clock: digest(if modality == Modality::Rgb { '1' } else { '2' }),
                optics: digest(if modality == Modality::Rgb { '3' } else { '4' }),
                extrinsics: digest(if modality == Modality::Rgb { '5' } else { '6' }),
                model: digest(prefix),
                training_data: digest(if modality == Modality::Rgb { '7' } else { '8' }),
                target_geometry: digest(if modality == Modality::Rgb { '9' } else { '0' }),
                renderer: digest(if modality == Modality::Rgb { 'c' } else { 'd' }),
                annotation: digest(if modality == Modality::Rgb { 'e' } else { 'f' }),
                translator: digest(if modality == Modality::Rgb { 'a' } else { 'b' }),
                declarations_complete: true,
            },
        }
    }

    fn policy() -> GatePolicy {
        GatePolicy {
            maximum_timestamp_skew_ms: 2,
            maximum_translation_disagreement_um: 5_000,
            maximum_orientation_chord_q30: 1_000,
            require_complete_dependency_declarations: true,
            shared_dependencies_fail_closed: true,
        }
    }

    #[test]
    fn independent_scope_consistent_pair_is_only_candidate() {
        let receipt = evaluate_pair(
            &estimate(Modality::Rgb, 1_000),
            &estimate(Modality::Event, 1_100),
            &policy(),
        )
        .expect("valid pair");
        assert_eq!(receipt.disposition, "CANDIDATE_CONSISTENT");
        assert_eq!(receipt.translation_disagreement_um, Some(100));
        assert_eq!(receipt.orientation_chord_q30, Some(0));
        assert!(receipt.shared_dependency_kinds.is_empty());
        assert!(!receipt.independence_proven);
        assert!(!receipt.physical_truth_established);
        assert!(!receipt.maneuver_authority_granted);
    }

    #[test]
    fn shared_clock_dependency_blocks_consistency_promotion() {
        let rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.dependencies.clock = rgb.dependencies.clock.clone();
        let receipt = evaluate_pair(&rgb, &event, &policy()).expect("valid pair");
        assert_eq!(receipt.disposition, "DEPENDENCE_UNKNOWN");
        assert_eq!(receipt.shared_dependency_kinds, ["clock"]);
    }

    #[test]
    fn shared_target_geometry_and_training_data_are_reported() {
        let mut rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.dependencies.target_geometry = rgb.dependencies.target_geometry.clone();
        event.dependencies.training_data = rgb.dependencies.training_data.clone();
        let receipt = evaluate_pair(&rgb, &event, &policy()).expect("valid pair");
        assert_eq!(receipt.disposition, "DEPENDENCE_UNKNOWN");
        assert_eq!(receipt.shared_dependency_kinds, ["target_geometry", "training_data"]);
    }

    #[test]
    fn common_mode_bias_can_agree_without_being_promoted_as_independent() {
        let rgb = estimate(Modality::Rgb, 100_000);
        let mut event = estimate(Modality::Event, 100_100);
        event.dependencies.target_geometry = rgb.dependencies.target_geometry.clone();
        let receipt = evaluate_pair(&rgb, &event, &policy()).expect("valid pair");
        assert_eq!(receipt.disposition, "DEPENDENCE_UNKNOWN");
        assert_eq!(receipt.translation_disagreement_um, Some(100));
    }

    #[test]
    fn conflicting_poses_are_rejected_even_when_dependencies_are_distinct() {
        let rgb = estimate(Modality::Rgb, 0);
        let event = estimate(Modality::Event, 50_000);
        let receipt = evaluate_pair(&rgb, &event, &policy()).expect("valid pair");
        assert_eq!(receipt.disposition, "DISAGREEMENT");
        assert!(receipt.reason_codes.contains(&"POSE_DISAGREEMENT_EXCEEDS_POLICY".to_owned()));
    }

    #[test]
    fn quaternion_sign_equivalence_does_not_create_false_conflict() {
        let rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.quaternion_xyzw_q30[3] = -Q30_ONE;
        let receipt = evaluate_pair(&rgb, &event, &policy()).expect("valid pair");
        assert_eq!(receipt.disposition, "CANDIDATE_CONSISTENT");
        assert_eq!(receipt.orientation_chord_q30, Some(0));
    }

    #[test]
    fn stale_timestamps_are_out_of_scope() {
        let rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.timestamp_ms += 10;
        let receipt = evaluate_pair(&rgb, &event, &policy()).expect("valid pair");
        assert_eq!(receipt.disposition, "OUT_OF_SCOPE");
        assert!(receipt.reason_codes.contains(&"TIMESTAMP_SKEW_EXCEEDS_POLICY".to_owned()));
    }

    #[test]
    fn mismatched_frame_or_coordinate_convention_is_out_of_scope() {
        let rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.frame_id = "frame-002".to_owned();
        assert_eq!(
            evaluate_pair(&rgb, &event, &policy()).expect("valid pair").disposition,
            "OUT_OF_SCOPE"
        );
        let mut event = estimate(Modality::Event, 0);
        event.frame_convention = "CAMERA_RIGHT_HANDED".to_owned();
        assert_eq!(
            evaluate_pair(&rgb, &event, &policy()).expect("valid pair").disposition,
            "OUT_OF_SCOPE"
        );
    }

    #[test]
    fn ood_and_low_observability_do_not_silently_fall_back_to_other_sensor() {
        let rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.ood = true;
        assert_eq!(
            evaluate_pair(&rgb, &event, &policy()).expect("valid pair").disposition,
            "ABSTAIN_MODALITY_UNAVAILABLE"
        );
        event.ood = false;
        event.low_observability = true;
        assert_eq!(
            evaluate_pair(&rgb, &event, &policy()).expect("valid pair").disposition,
            "ABSTAIN_LOW_OBSERVABILITY"
        );
    }

    #[test]
    fn incomplete_dependency_declarations_are_unknown() {
        let rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.dependencies.declarations_complete = false;
        let receipt = evaluate_pair(&rgb, &event, &policy()).expect("valid pair");
        assert_eq!(receipt.disposition, "DEPENDENCE_UNKNOWN");
    }

    #[test]
    fn malformed_digest_and_non_unit_quaternion_are_rejected() {
        let mut rgb = estimate(Modality::Rgb, 0);
        rgb.dependencies.clock = "bad".to_owned();
        assert!(evaluate_pair(&rgb, &estimate(Modality::Event, 0), &policy()).is_err());
        let rgb = estimate(Modality::Rgb, 0);
        let mut event = estimate(Modality::Event, 0);
        event.quaternion_xyzw_q30 = [0, 0, 0, 0];
        assert!(evaluate_pair(&rgb, &event, &policy()).is_err());
    }
}
