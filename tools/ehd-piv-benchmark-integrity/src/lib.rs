//! Leakage-resistant cohort and split audit for PIV validation.
//!
//! This crate verifies metadata supplied by a benchmark curator. It does not
//! hash files, inspect images, infer acquisition independence, or decide which
//! holdout dimensions are scientifically appropriate. Those are explicit
//! responsibilities of the frozen benchmark manifest.
//!
//! For every declared holdout dimension, values must not occur in more than
//! one of train/validation/test. Raw five-frame sequences or image-pair hashes
//! must be unique throughout the entire cohort. This prevents direct image
//! reuse and selected forms of session/geometry/flow-family leakage; it does
//! not guarantee absence of subtler correlations or distribution leakage.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatasetSplit {
    Train,
    Validation,
    Test,
}

impl DatasetSplit {
    pub const ALL: [Self; 3] = [Self::Train, Self::Validation, Self::Test];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HoldoutDimension {
    FlowFamily,
    Geometry,
    Acquisition,
    Session,
    CameraCalibrationBatch,
    Tracer,
    PressureRegime,
}

impl HoldoutDimension {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FlowFamily => "flow_family",
            Self::Geometry => "geometry",
            Self::Acquisition => "acquisition",
            Self::Session => "session",
            Self::CameraCalibrationBatch => "camera_calibration_batch",
            Self::Tracer => "tracer",
            Self::PressureRegime => "pressure_regime",
        }
    }

    fn value<'a>(self, sample: &'a SampleRecord) -> &'a str {
        match self {
            Self::FlowFamily => &sample.flow_family_id,
            Self::Geometry => &sample.geometry_id,
            Self::Acquisition => &sample.acquisition_id,
            Self::Session => &sample.session_id,
            Self::CameraCalibrationBatch => &sample.camera_calibration_batch_id,
            Self::Tracer => &sample.tracer_id,
            Self::PressureRegime => &sample.pressure_regime_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleRecord {
    pub sample_id: String,
    pub source_dataset_id: String,
    pub split: DatasetSplit,
    pub flow_family_id: String,
    pub geometry_id: String,
    pub acquisition_id: String,
    pub session_id: String,
    pub camera_calibration_batch_id: String,
    pub tracer_id: String,
    pub pressure_regime_id: String,
    /// SHA-256 of the raw image pair or complete multi-frame sequence bytes.
    pub raw_sequence_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitAuditInput {
    pub manifest_id: String,
    pub source_dataset_id: String,
    pub policy_id: String,
    /// Dimensions that must be disjoint across train, validation and test.
    pub held_out_dimensions: Vec<HoldoutDimension>,
    pub require_unique_raw_sequence_hashes: bool,
    pub samples: Vec<SampleRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitCount {
    pub split: DatasetSplit,
    pub sample_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldoutGroupCount {
    pub dimension: HoldoutDimension,
    pub unique_group_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeakageViolation {
    pub code: &'static str,
    pub dimension: Option<HoldoutDimension>,
    pub value: String,
    pub splits: Vec<DatasetSplit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitAuditReport {
    pub manifest_id: String,
    pub source_dataset_id: String,
    pub policy_id: String,
    pub sample_count: usize,
    pub split_counts: Vec<SplitCount>,
    pub holdout_group_counts: Vec<HoldoutGroupCount>,
    pub violations: Vec<LeakageViolation>,
    pub passed: bool,
    pub limitations: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SplitAuditError {
    EmptyIdentity { field: &'static str },
    EmptySamples,
    NoHoldoutDimensions,
    DuplicateHoldoutDimension { dimension: HoldoutDimension },
    DuplicateSampleId { sample_id: String },
    SourceDatasetMismatch { sample_id: String },
    MissingDimensionValue { sample_id: String, dimension: HoldoutDimension },
    InvalidSha256 { sample_id: String },
    MissingSplit { split: DatasetSplit },
}

impl fmt::Display for SplitAuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "PIV split audit {field} must be non-empty"),
            Self::EmptySamples => write!(f, "PIV split audit requires sample records"),
            Self::NoHoldoutDimensions => write!(f, "at least one explicit holdout dimension is required"),
            Self::DuplicateHoldoutDimension { dimension } => {
                write!(f, "duplicate holdout dimension: {}", dimension.as_str())
            }
            Self::DuplicateSampleId { sample_id } => write!(f, "duplicate sample ID: {sample_id}"),
            Self::SourceDatasetMismatch { sample_id } => {
                write!(f, "sample {sample_id} does not match the declared source dataset")
            }
            Self::MissingDimensionValue { sample_id, dimension } => {
                write!(f, "sample {sample_id} is missing {}", dimension.as_str())
            }
            Self::InvalidSha256 { sample_id } => {
                write!(f, "sample {sample_id} has an invalid raw-sequence SHA-256")
            }
            Self::MissingSplit { split } => write!(f, "required split is empty: {split:?}"),
        }
    }
}

impl Error for SplitAuditError {}

fn validate_identity(field: &'static str, value: &str) -> Result<(), SplitAuditError> {
    if value.trim().is_empty() {
        return Err(SplitAuditError::EmptyIdentity { field });
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Audit explicit train/validation/test records for split leakage.
///
/// Every declared holdout dimension must be disjoint across all three splits.
/// This is deliberately strict: if a benchmark does not intend to hold out a
/// dimension (for example, flow family), it must omit that dimension from the
/// frozen policy rather than relying on an implicit exception.
pub fn audit_split_integrity(
    input: &SplitAuditInput,
) -> Result<SplitAuditReport, SplitAuditError> {
    validate_identity("manifest_id", &input.manifest_id)?;
    validate_identity("source_dataset_id", &input.source_dataset_id)?;
    validate_identity("policy_id", &input.policy_id)?;

    if input.samples.is_empty() {
        return Err(SplitAuditError::EmptySamples);
    }
    if input.held_out_dimensions.is_empty() {
        return Err(SplitAuditError::NoHoldoutDimensions);
    }

    let mut dimensions = BTreeSet::new();
    for dimension in &input.held_out_dimensions {
        if !dimensions.insert(*dimension) {
            return Err(SplitAuditError::DuplicateHoldoutDimension {
                dimension: *dimension,
            });
        }
    }

    let mut sample_ids = BTreeSet::new();
    let mut split_counts = BTreeMap::<DatasetSplit, usize>::new();
    let mut dimension_values =
        BTreeMap::<HoldoutDimension, BTreeMap<String, BTreeSet<DatasetSplit>>>::new();
    let mut raw_hashes =
        BTreeMap::<String, (BTreeSet<DatasetSplit>, usize)>::new();

    for sample in &input.samples {
        validate_identity("sample_id", &sample.sample_id)?;
        if !sample_ids.insert(sample.sample_id.clone()) {
            return Err(SplitAuditError::DuplicateSampleId {
                sample_id: sample.sample_id.clone(),
            });
        }
        if sample.source_dataset_id != input.source_dataset_id {
            return Err(SplitAuditError::SourceDatasetMismatch {
                sample_id: sample.sample_id.clone(),
            });
        }
        if !valid_sha256(&sample.raw_sequence_sha256) {
            return Err(SplitAuditError::InvalidSha256 {
                sample_id: sample.sample_id.clone(),
            });
        }

        *split_counts.entry(sample.split).or_default() += 1;
        let hash_entry = raw_hashes
            .entry(sample.raw_sequence_sha256.to_ascii_lowercase())
            .or_insert_with(|| (BTreeSet::new(), 0));
        hash_entry.0.insert(sample.split);
        hash_entry.1 += 1;

        for dimension in &input.held_out_dimensions {
            let value = dimension.value(sample);
            if value.trim().is_empty() {
                return Err(SplitAuditError::MissingDimensionValue {
                    sample_id: sample.sample_id.clone(),
                    dimension: *dimension,
                });
            }
            dimension_values
                .entry(*dimension)
                .or_default()
                .entry(value.to_owned())
                .or_default()
                .insert(sample.split);
        }
    }

    for split in DatasetSplit::ALL {
        if split_counts.get(&split).copied().unwrap_or(0) == 0 {
            return Err(SplitAuditError::MissingSplit { split });
        }
    }

    let mut violations = Vec::new();

    for (dimension, values) in &dimension_values {
        for (value, splits) in values {
            if splits.len() > 1 {
                violations.push(LeakageViolation {
                    code: "HOLDOUT_DIMENSION_CROSSES_SPLITS",
                    dimension: Some(*dimension),
                    value: value.clone(),
                    splits: splits.iter().copied().collect(),
                });
            }
        }
    }

    if input.require_unique_raw_sequence_hashes {
        for (hash, (splits, occurrences)) in &raw_hashes {
            if *occurrences > 1 {
                violations.push(LeakageViolation {
                    code: "RAW_SEQUENCE_HASH_REUSED",
                    dimension: None,
                    value: hash.clone(),
                    splits: splits.iter().copied().collect(),
                });
            }
        }
    }

    let split_counts = DatasetSplit::ALL
        .into_iter()
        .map(|split| SplitCount {
            split,
            sample_count: split_counts.get(&split).copied().unwrap_or(0),
        })
        .collect();

    let holdout_group_counts = input
        .held_out_dimensions
        .iter()
        .map(|dimension| HoldoutGroupCount {
            dimension: *dimension,
            unique_group_count: dimension_values
                .get(dimension)
                .map(BTreeMap::len)
                .unwrap_or(0),
        })
        .collect();

    Ok(SplitAuditReport {
        manifest_id: input.manifest_id.clone(),
        source_dataset_id: input.source_dataset_id.clone(),
        policy_id: input.policy_id.clone(),
        sample_count: input.samples.len(),
        split_counts,
        holdout_group_counts,
        passed: violations.is_empty(),
        violations,
        limitations: "Metadata-only split audit. The curator must choose scientifically meaningful holdout dimensions and compute SHA-256 over the correct raw sequence bytes. Passing proves only the declared group/hash disjointness; it does not establish statistical independence, representative coverage, ground-truth quality, image integrity, or absence of unrecorded common causes.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        index: usize,
        split: DatasetSplit,
        family: &str,
        geometry: &str,
        session: &str,
    ) -> SampleRecord {
        SampleRecord {
            sample_id: format!("sample-{index}"),
            source_dataset_id: "FFS-S-PIV-v1".to_owned(),
            split,
            flow_family_id: family.to_owned(),
            geometry_id: geometry.to_owned(),
            acquisition_id: format!("acq-{index}"),
            session_id: session.to_owned(),
            camera_calibration_batch_id: format!("cal-{index}"),
            tracer_id: format!("tracer-{index}"),
            pressure_regime_id: format!("pressure-{index}"),
            raw_sequence_sha256: format!("{index:064x}"),
        }
    }

    fn clean_input() -> SplitAuditInput {
        SplitAuditInput {
            manifest_id: "FFS-S-PIV-TEST-MANIFEST".to_owned(),
            source_dataset_id: "FFS-S-PIV-v1".to_owned(),
            policy_id: "leave-session-geometry-family-out-v1".to_owned(),
            held_out_dimensions: vec![
                HoldoutDimension::FlowFamily,
                HoldoutDimension::Geometry,
                HoldoutDimension::Session,
            ],
            require_unique_raw_sequence_hashes: true,
            samples: vec![
                sample(1, DatasetSplit::Train, "channel", "planar", "session-train"),
                sample(2, DatasetSplit::Validation, "backstep", "stereo", "session-val"),
                sample(3, DatasetSplit::Test, "mhd", "tomographic", "session-test"),
            ],
        }
    }

    #[test]
    fn disjoint_family_geometry_and_session_holdout_passes() {
        let report = audit_split_integrity(&clean_input()).unwrap();
        assert!(report.passed);
        assert_eq!(report.sample_count, 3);
        assert_eq!(report.violations.len(), 0);
    }

    #[test]
    fn session_leakage_across_train_and_test_is_detected() {
        let mut input = clean_input();
        input.samples[2].session_id = input.samples[0].session_id.clone();
        let report = audit_split_integrity(&input).unwrap();
        assert!(!report.passed);
        assert!(report.violations.iter().any(|violation| {
            violation.dimension == Some(HoldoutDimension::Session)
                && violation.code == "HOLDOUT_DIMENSION_CROSSES_SPLITS"
        }));
    }

    #[test]
    fn geometry_leakage_is_detected_when_geometry_is_a_holdout_dimension() {
        let mut input = clean_input();
        input.samples[2].geometry_id = input.samples[0].geometry_id.clone();
        let report = audit_split_integrity(&input).unwrap();
        assert!(report.violations.iter().any(|violation| {
            violation.dimension == Some(HoldoutDimension::Geometry)
        }));
    }

    #[test]
    fn direct_raw_sequence_reuse_is_detected_even_without_group_overlap() {
        let mut input = clean_input();
        input.samples[2].raw_sequence_sha256 = input.samples[0].raw_sequence_sha256.clone();
        let report = audit_split_integrity(&input).unwrap();
        assert!(report.violations.iter().any(|violation| {
            violation.code == "RAW_SEQUENCE_HASH_REUSED"
        }));
    }

    #[test]
    fn non_held_out_flow_family_is_not_implicitly_rejected() {
        let mut input = clean_input();
        input.held_out_dimensions = vec![HoldoutDimension::Session];
        input.samples[2].flow_family_id = input.samples[0].flow_family_id.clone();
        let report = audit_split_integrity(&input).unwrap();
        assert!(report.passed);
    }

    #[test]
    fn duplicate_sample_identity_is_rejected() {
        let mut input = clean_input();
        input.samples[2].sample_id = input.samples[0].sample_id.clone();
        assert!(matches!(
            audit_split_integrity(&input),
            Err(SplitAuditError::DuplicateSampleId { .. })
        ));
    }

    #[test]
    fn missing_held_out_dimension_value_is_rejected() {
        let mut input = clean_input();
        input.samples[2].camera_calibration_batch_id.clear();
        input.held_out_dimensions.push(HoldoutDimension::CameraCalibrationBatch);
        assert!(matches!(
            audit_split_integrity(&input),
            Err(SplitAuditError::MissingDimensionValue {
                dimension: HoldoutDimension::CameraCalibrationBatch,
                ..
            })
        ));
    }

    #[test]
    fn split_without_records_is_rejected() {
        let mut input = clean_input();
        input.samples.pop();
        assert!(matches!(
            audit_split_integrity(&input),
            Err(SplitAuditError::MissingSplit {
                split: DatasetSplit::Test
            })
        ));
    }

    #[test]
    fn duplicate_raw_hash_inside_one_split_is_also_rejected() {
        let mut input = clean_input();
        let mut additional_train = sample(
            4,
            DatasetSplit::Train,
            "wake",
            "planar-offset",
            "session-train-2",
        );
        additional_train.raw_sequence_sha256 = input.samples[0].raw_sequence_sha256.clone();
        input.samples.push(additional_train);
        let report = audit_split_integrity(&input).unwrap();
        assert!(report.violations.iter().any(|violation| {
            violation.code == "RAW_SEQUENCE_HASH_REUSED"
                && violation.splits == vec![DatasetSplit::Train]
        }));
    }

    #[test]
    fn duplicate_held_out_dimension_is_rejected() {
        let mut input = clean_input();
        input.held_out_dimensions.push(HoldoutDimension::Session);
        assert!(matches!(
            audit_split_integrity(&input),
            Err(SplitAuditError::DuplicateHoldoutDimension {
                dimension: HoldoutDimension::Session
            })
        ));
    }
}
