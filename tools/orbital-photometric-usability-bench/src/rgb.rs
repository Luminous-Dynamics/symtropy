use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const RGB_CORPUS_SCHEMA: &str = "luminous.orbital-photometric-usability.rgb-corpus.v0.2";
pub const RGB_RECEIPT_SCHEMA: &str = "luminous.orbital-photometric-usability.rgb-receipt.v0.2";
pub const RGB_BENCHMARK_ID: &str = "ORBITAL-PHOTO-RGB-002";
pub const RGB_CLAIM_CEILING: &str =
    "synthetic_rgb_range_and_declared_camera_projector_preconditions_only";
pub const RGB_CASE_COUNT: usize = 10;
const RGB_SCALE: f64 = 1_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Gate {
    Pass,
    Fail,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RgbStatus {
    Usable,
    Underexposed,
    Saturated,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JointStatus {
    Usable,
    GeometryRejected,
    Underexposed,
    Saturated,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModeSummary {
    FullyUsable,
    Partial,
    NotUsable,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Acquisition {
    Usable,
    Partial,
    Unusable,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RgbMode {
    pub mode_id: String,
    /// Normalized linear RGB darkness threshold. At least one channel must be >= this.
    pub darkness_threshold: f64,
    /// Normalized linear RGB saturation ceiling. Every channel must be <= this.
    pub saturation_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RgbSample {
    pub sample_id: String,
    /// Upstream geometry outcomes; this crate does not recompute them.
    pub camera_fov: Gate,
    pub projector_fov: Gate,
    pub camera_depth_of_field: Gate,
    pub camera_line_of_sight: Gate,
    /// Caller-supplied synthetic linear RGB response, each channel in [0, 1].
    pub rgb_linear: [f64; 3],
    pub rgb_response_asserted: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedMode {
    pub mode_id: String,
    pub summary: ModeSummary,
    pub acquisition: Acquisition,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RgbCase {
    pub case_id: String,
    pub camera_id: String,
    pub projector_id: String,
    pub samples: Vec<RgbSample>,
    pub modes: Vec<RgbMode>,
    pub expected: Vec<ExpectedMode>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RgbCorpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<RgbCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RgbSampleResult {
    pub sample_id: String,
    pub geometry_gate: Gate,
    pub rgb_micro_units: Option<[u64; 3]>,
    pub rgb_status: RgbStatus,
    pub joint_status: JointStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RgbModeResult {
    pub mode_id: String,
    pub sample_count: usize,
    pub usable_count: usize,
    pub geometry_rejected_count: usize,
    pub underexposed_count: usize,
    pub saturated_count: usize,
    pub indeterminate_count: usize,
    pub summary: ModeSummary,
    pub acquisition: Acquisition,
    pub samples: Vec<RgbSampleResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RgbCaseResult {
    pub case_id: String,
    pub expected: Vec<ExpectedMode>,
    pub expected_matches: usize,
    pub expected_mismatches: usize,
    pub modes: Vec<RgbModeResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RgbReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub fixture_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_matches: usize,
    pub expected_mismatches: usize,
    pub authority_granted: bool,
    pub structural_acceptance_authorized: bool,
    pub physical_validation: String,
    pub cases: Vec<RgbCaseResult>,
}

fn geometry_gate(sample: &RgbSample) -> Gate {
    let gates = [
        sample.camera_fov,
        sample.projector_fov,
        sample.camera_depth_of_field,
        sample.camera_line_of_sight,
    ];
    if gates.contains(&Gate::Fail) {
        Gate::Fail
    } else if gates.contains(&Gate::Indeterminate) {
        Gate::Indeterminate
    } else {
        Gate::Pass
    }
}

fn quantize_rgb(rgb: [f64; 3]) -> Result<[u64; 3], String> {
    let mut result = [0_u64; 3];
    for (index, channel) in rgb.into_iter().enumerate() {
        if !channel.is_finite() || !(0.0..=1.0).contains(&channel) {
            return Err("RGB_CHANNEL_OUT_OF_RANGE".to_owned());
        }
        let scaled = (channel * RGB_SCALE + 0.5).floor();
        if !scaled.is_finite() || scaled > u64::MAX as f64 {
            return Err("RGB_CHANNEL_OVERFLOW".to_owned());
        }
        result[index] = scaled as u64;
    }
    Ok(result)
}

fn quantize_threshold(value: f64) -> Result<u64, String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err("RGB_THRESHOLD_OUT_OF_RANGE".to_owned());
    }
    Ok((value * RGB_SCALE + 0.5).floor() as u64)
}

fn validate_case(case: &RgbCase) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.camera_id.trim().is_empty()
        || case.projector_id.trim().is_empty()
        || case.samples.is_empty()
        || case.modes.is_empty()
    {
        return Err("RGB_CASE_BOUNDS_INVALID".to_owned());
    }

    let mut sample_ids = BTreeSet::new();
    for sample in &case.samples {
        if sample.sample_id.trim().is_empty() || !sample_ids.insert(sample.sample_id.as_str()) {
            return Err("RGB_SAMPLE_ID_INVALID".to_owned());
        }
        quantize_rgb(sample.rgb_linear)?;
    }

    let mut mode_ids = BTreeSet::new();
    for mode in &case.modes {
        if mode.mode_id.trim().is_empty() || !mode_ids.insert(mode.mode_id.as_str()) {
            return Err("RGB_MODE_ID_INVALID".to_owned());
        }
        let darkness = quantize_threshold(mode.darkness_threshold)?;
        let saturation = quantize_threshold(mode.saturation_threshold)?;
        if darkness >= saturation {
            return Err("RGB_THRESHOLD_ORDER_INVALID".to_owned());
        }
    }

    let expected_ids: BTreeSet<&str> = case.expected.iter().map(|item| item.mode_id.as_str()).collect();
    if expected_ids.len() != case.expected.len() || expected_ids != mode_ids {
        return Err("RGB_EXPECTED_MODE_SET_INVALID".to_owned());
    }
    Ok(())
}

fn evaluate_sample(sample: &RgbSample, mode: &RgbMode) -> Result<RgbSampleResult, String> {
    let geometry = geometry_gate(sample);
    if !sample.rgb_response_asserted {
        return Ok(RgbSampleResult {
            sample_id: sample.sample_id.clone(),
            geometry_gate: geometry,
            rgb_micro_units: None,
            rgb_status: RgbStatus::Indeterminate,
            joint_status: JointStatus::Indeterminate,
        });
    }

    let rgb = quantize_rgb(sample.rgb_linear)?;
    let darkness = quantize_threshold(mode.darkness_threshold)?;
    let saturation = quantize_threshold(mode.saturation_threshold)?;
    let any_channel_above_darkness = rgb.iter().any(|value| *value >= darkness);
    let any_channel_above_saturation = rgb.iter().any(|value| *value > saturation);

    let rgb_status = if any_channel_above_saturation {
        RgbStatus::Saturated
    } else if !any_channel_above_darkness {
        RgbStatus::Underexposed
    } else {
        RgbStatus::Usable
    };

    let joint_status = match geometry {
        Gate::Fail => JointStatus::GeometryRejected,
        Gate::Indeterminate => JointStatus::Indeterminate,
        Gate::Pass => match rgb_status {
            RgbStatus::Usable => JointStatus::Usable,
            RgbStatus::Underexposed => JointStatus::Underexposed,
            RgbStatus::Saturated => JointStatus::Saturated,
            RgbStatus::Indeterminate => JointStatus::Indeterminate,
        },
    };

    Ok(RgbSampleResult {
        sample_id: sample.sample_id.clone(),
        geometry_gate: geometry,
        rgb_micro_units: Some(rgb),
        rgb_status,
        joint_status,
    })
}

fn evaluate_mode(case: &RgbCase, mode: &RgbMode) -> Result<RgbModeResult, String> {
    let samples = case.samples.iter()
        .map(|sample| evaluate_sample(sample, mode))
        .collect::<Result<Vec<_>, _>>()?;
    let count = samples.len();
    let usable = samples.iter().filter(|sample| sample.joint_status == JointStatus::Usable).count();
    let geometry_rejected = samples.iter().filter(|sample| sample.joint_status == JointStatus::GeometryRejected).count();
    let underexposed = samples.iter().filter(|sample| sample.joint_status == JointStatus::Underexposed).count();
    let saturated = samples.iter().filter(|sample| sample.joint_status == JointStatus::Saturated).count();
    let indeterminate = samples.iter().filter(|sample| sample.joint_status == JointStatus::Indeterminate).count();

    let summary = if indeterminate > 0 {
        ModeSummary::Indeterminate
    } else if usable == count {
        ModeSummary::FullyUsable
    } else if usable > 0 {
        ModeSummary::Partial
    } else {
        ModeSummary::NotUsable
    };
    let acquisition = match summary {
        ModeSummary::FullyUsable => Acquisition::Usable,
        ModeSummary::Partial => Acquisition::Partial,
        ModeSummary::NotUsable => Acquisition::Unusable,
        ModeSummary::Indeterminate => Acquisition::Indeterminate,
    };

    Ok(RgbModeResult {
        mode_id: mode.mode_id.clone(),
        sample_count: count,
        usable_count: usable,
        geometry_rejected_count: geometry_rejected,
        underexposed_count: underexposed,
        saturated_count: saturated,
        indeterminate_count: indeterminate,
        summary,
        acquisition,
        samples,
    })
}

pub fn evaluate_case(case: &RgbCase) -> Result<RgbCaseResult, String> {
    validate_case(case)?;
    let modes = case.modes.iter()
        .map(|mode| evaluate_mode(case, mode))
        .collect::<Result<Vec<_>, _>>()?;
    let mut expected_matches = 0_usize;
    for expected in &case.expected {
        let actual = modes.iter().find(|mode| mode.mode_id == expected.mode_id)
            .ok_or_else(|| "RGB_EXPECTED_MODE_MISSING".to_owned())?;
        if actual.summary == expected.summary && actual.acquisition == expected.acquisition {
            expected_matches += 1;
        }
    }
    Ok(RgbCaseResult {
        case_id: case.case_id.clone(),
        expected: case.expected.clone(),
        expected_matches,
        expected_mismatches: case.expected.len() - expected_matches,
        modes,
    })
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn audit_rgb_corpus(bytes: &[u8], source_commit: &str) -> Result<RgbReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_INVALID".to_owned());
    }
    let corpus: RgbCorpus = serde_json::from_slice(bytes)
        .map_err(|error| format!("RGB_CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != RGB_CORPUS_SCHEMA
        || corpus.benchmark_id != RGB_BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != RGB_CLAIM_CEILING
        || corpus.cases.len() != RGB_CASE_COUNT
    {
        return Err("RGB_CORPUS_IDENTITY_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("RGB_DUPLICATE_CASE_ID".to_owned());
        }
        cases.push(evaluate_case(case)?);
    }
    let expected_matches = cases.iter().map(|case| case.expected_matches).sum::<usize>();
    let expected_total = cases.iter().map(|case| case.expected.len()).sum::<usize>();

    Ok(RgbReceipt {
        schema: RGB_RECEIPT_SCHEMA.to_owned(),
        benchmark_id: RGB_BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        fixture_sha256: digest(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_matches,
        expected_mismatches: expected_total - expected_matches,
        authority_granted: false,
        structural_acceptance_authorized: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, rgb: [f64; 3]) -> RgbSample {
        RgbSample {
            sample_id: id.to_owned(),
            camera_fov: Gate::Pass,
            projector_fov: Gate::Pass,
            camera_depth_of_field: Gate::Pass,
            camera_line_of_sight: Gate::Pass,
            rgb_linear: rgb,
            rgb_response_asserted: true,
        }
    }

    fn mode() -> RgbMode {
        RgbMode {
            mode_id: "MODE-A".to_owned(),
            darkness_threshold: 0.1,
            saturation_threshold: 0.9,
        }
    }

    fn case(samples: Vec<RgbSample>) -> RgbCase {
        RgbCase {
            case_id: "RGB-UNIT-TEST".to_owned(),
            camera_id: "CAMERA-A".to_owned(),
            projector_id: "PROJECTOR-A".to_owned(),
            samples,
            modes: vec![mode()],
            expected: vec![ExpectedMode {
                mode_id: "MODE-A".to_owned(),
                summary: ModeSummary::FullyUsable,
                acquisition: Acquisition::Usable,
            }],
        }
    }

    #[test]
    fn normal_rgb_is_usable() {
        let result = evaluate_case(&case(vec![sample("S1", [0.5, 0.4, 0.2])])).unwrap();
        assert_eq!(result.modes[0].samples[0].joint_status, JointStatus::Usable);
    }

    #[test]
    fn any_channel_above_saturation_rejects_rgb() {
        let result = evaluate_case(&case(vec![sample("S1", [0.91, 0.3, 0.2])])).unwrap();
        assert_eq!(result.modes[0].samples[0].rgb_status, RgbStatus::Saturated);
        assert_eq!(result.modes[0].samples[0].joint_status, JointStatus::Saturated);
    }

    #[test]
    fn every_channel_below_darkness_is_underexposed() {
        let result = evaluate_case(&case(vec![sample("S1", [0.01, 0.09, 0.05])])).unwrap();
        assert_eq!(result.modes[0].samples[0].rgb_status, RgbStatus::Underexposed);
    }

    #[test]
    fn darkness_and_saturation_boundaries_are_inclusive() {
        let result = evaluate_case(&case(vec![sample("S1", [0.1, 0.9, 0.4])])).unwrap();
        assert_eq!(result.modes[0].samples[0].rgb_status, RgbStatus::Usable);
    }

    #[test]
    fn projector_fov_failure_rejects_even_usable_rgb() {
        let mut sample = sample("S1", [0.5, 0.4, 0.2]);
        sample.projector_fov = Gate::Fail;
        let result = evaluate_case(&case(vec![sample])).unwrap();
        assert_eq!(result.modes[0].samples[0].rgb_status, RgbStatus::Usable);
        assert_eq!(result.modes[0].samples[0].joint_status, JointStatus::GeometryRejected);
    }

    #[test]
    fn indeterminate_precondition_does_not_become_usable() {
        let mut sample = sample("S1", [0.5, 0.4, 0.2]);
        sample.camera_depth_of_field = Gate::Indeterminate;
        let result = evaluate_case(&case(vec![sample])).unwrap();
        assert_eq!(result.modes[0].samples[0].joint_status, JointStatus::Indeterminate);
    }

    #[test]
    fn unasserted_rgb_response_abstains() {
        let mut sample = sample("S1", [0.5, 0.4, 0.2]);
        sample.rgb_response_asserted = false;
        let result = evaluate_case(&case(vec![sample])).unwrap();
        assert_eq!(result.modes[0].samples[0].rgb_status, RgbStatus::Indeterminate);
    }

    #[test]
    fn mixed_samples_are_partial_not_fully_usable() {
        let result = evaluate_case(&case(vec![
            sample("GOOD", [0.5, 0.4, 0.2]),
            sample("DARK", [0.01, 0.02, 0.03]),
        ])).unwrap();
        assert_eq!(result.modes[0].summary, ModeSummary::Partial);
        assert_eq!(result.modes[0].acquisition, Acquisition::Partial);
    }

    #[test]
    fn geometry_precondition_rotation_semantics_are_consumed_not_recomputed() {
        let mut sample = sample("S1", [0.5, 0.4, 0.2]);
        sample.camera_fov = Gate::Fail;
        let result = evaluate_case(&case(vec![sample])).unwrap();
        assert_eq!(result.modes[0].samples[0].geometry_gate, Gate::Fail);
    }
}
