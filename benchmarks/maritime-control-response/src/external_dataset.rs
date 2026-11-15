#![forbid(unsafe_code)]

//! Source-bound manifest validation for the separate Marine Engine Fault
//! dataset (Zenodo 10.5281/zenodo.19857425).
//!
//! This module does not parse CSV payloads or claim the archive has been
//! inspected. It validates a caller-produced manifest after the archive,
//! three-row headers, variable dictionary and file-level metadata are audited.
//! It keeps this dataset's label space separate from LiU-ICE.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MARINE_DATASET_DOI: &str = "10.5281/zenodo.19857425";
pub const MARINE_DATASET_SCHEMA_VERSION: &str = "symtropy-marine-file-manifest-v1";
pub const MARINE_DATASET_EXPECTED_CSV_FILES: usize = 16;
pub const MARINE_DATASET_EXPECTED_SCENARIO_FILES: usize = 15;
pub const MARINE_DATASET_EXPECTED_HEADER_ROWS: usize = 3;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MarineFaultFamily {
    CompressorAirFilterClogging,
    AirCoolerFouling,
    InjectionValveNozzleClogging,
    CoolingWaterPumpCavitation,
    TurbineDegradation,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarineCsvSchema {
    Baseline70,
    Scenario73,
}

impl MarineCsvSchema {
    pub fn expected_column_count(self) -> usize {
        match self {
            Self::Baseline70 => 70,
            Self::Scenario73 => 73,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MarineDatasetFile {
    pub file_id: String,
    pub relative_path: String,
    pub sha256: String,
    pub row_count: u64,
    pub schema: MarineCsvSchema,
    pub header_rows: usize,
    pub recorded_column_count: usize,
    pub role: MarineFileRole,
    pub fault_family: Option<MarineFaultFamily>,
    /// Load points represented in this recording, not a load estimate inferred from the signal.
    pub load_points_percent: Vec<u8>,
    pub anomaly_state_label_present: bool,
    pub contains_pre_anomaly_samples: bool,
    pub event_latency_eligible: bool,
    /// Empty fields declared by the audited file index/variable dictionary.
    pub missing_channels: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarineFileRole {
    BaselineReference,
    FaultScenario,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MarineDatasetSplit {
    Train,
    Calibration,
    Validation,
    Test,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MarineFileSplitAssignment {
    pub file_id: String,
    pub split: MarineDatasetSplit,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MarineFileSplitManifest {
    pub schema_version: u32,
    pub assignments: Vec<MarineFileSplitAssignment>,
}

impl MarineFileSplitManifest {
    pub fn validate_against(
        &self,
        files: &[MarineDatasetFile],
    ) -> Result<(), ExternalDatasetError> {
        if self.schema_version != 1 || self.assignments.is_empty() {
            return Err(ExternalDatasetError::InvalidSplitManifest);
        }
        let known_file_ids = files
            .iter()
            .map(|file| file.file_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut assigned = BTreeSet::new();
        let mut has_train = false;
        let mut has_test = false;
        for assignment in &self.assignments {
            if !known_file_ids.contains(assignment.file_id.as_str()) {
                return Err(ExternalDatasetError::UnknownSplitFile(
                    assignment.file_id.clone(),
                ));
            }
            if !assigned.insert(assignment.file_id.as_str()) {
                return Err(ExternalDatasetError::DuplicateSplitFile(
                    assignment.file_id.clone(),
                ));
            }
            has_train |= assignment.split == MarineDatasetSplit::Train;
            has_test |= assignment.split == MarineDatasetSplit::Test;
        }
        if assigned.len() != known_file_ids.len() || !has_train || !has_test {
            return Err(ExternalDatasetError::InvalidSplitManifest);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, ExternalDatasetError> {
        sha256_serializable(b"symtropy-marine-file-split-manifest-v1\0", self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MarineExternalDatasetManifest {
    pub schema_version: String,
    pub dataset_doi: String,
    pub dataset_license: String,
    pub archive_sha256: String,
    pub dataset_index_sha256: String,
    pub variable_dictionary_sha256: String,
    pub normalization_script_sha256: String,
    pub files: Vec<MarineDatasetFile>,
    pub split_manifest: Option<MarineFileSplitManifest>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct MarineDatasetAuditSummary {
    pub file_count: usize,
    pub baseline_file_count: usize,
    pub scenario_file_count: usize,
    pub missing_channel_file_count: usize,
    pub anomaly_only_file_count: usize,
    pub event_latency_eligible_file_count: usize,
    pub family_file_counts: BTreeMap<MarineFaultFamily, usize>,
    pub file_manifest_sha256: String,
    pub split_manifest_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalDatasetError {
    WrongDatasetIdentity,
    InvalidDigest(&'static str),
    WrongFileCount,
    EmptyFileIdentity,
    DuplicateFileIdentity(String),
    DuplicateFilePath(String),
    InvalidFileDigest(String),
    EmptyFile(String),
    WrongHeaderShape(String),
    WrongColumnCount(String),
    InvalidRoleSemantics(String),
    InvalidLoadPoint(String),
    InvalidMissingChannel(String),
    WrongBaselineCount,
    WrongScenarioCount,
    WrongFamilyCounts(MarineFaultFamily),
    WrongMissingChannelRunCount,
    WrongAnomalyOnlyFileCount,
    InvalidEventLatencyEligibility(String),
    InvalidSplitManifest,
    UnknownSplitFile(String),
    DuplicateSplitFile(String),
    Serialization,
}

impl fmt::Display for ExternalDatasetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongDatasetIdentity => write!(
                formatter,
                "dataset DOI, licence or manifest schema version is incorrect"
            ),
            Self::InvalidDigest(field) => {
                write!(formatter, "{field} must be a lowercase SHA-256 digest")
            }
            Self::WrongFileCount => write!(
                formatter,
                "dataset must contain exactly 16 CSV file entries"
            ),
            Self::EmptyFileIdentity => write!(
                formatter,
                "file identity and relative path must be non-empty"
            ),
            Self::DuplicateFileIdentity(id) => {
                write!(formatter, "duplicate dataset file identity: {id}")
            }
            Self::DuplicateFilePath(path) => {
                write!(formatter, "duplicate dataset file path: {path}")
            }
            Self::InvalidFileDigest(id) => write!(formatter, "invalid file SHA-256 digest: {id}"),
            Self::EmptyFile(id) => write!(formatter, "file has no parsed data rows: {id}"),
            Self::WrongHeaderShape(id) => write!(
                formatter,
                "file must have the published three-row header: {id}"
            ),
            Self::WrongColumnCount(id) => write!(
                formatter,
                "file column count does not match its declared baseline/scenario schema: {id}"
            ),
            Self::InvalidRoleSemantics(id) => write!(
                formatter,
                "baseline/scenario role, fault family or anomaly labels are inconsistent: {id}"
            ),
            Self::InvalidLoadPoint(id) => write!(
                formatter,
                "load points must be supported values between 30 and 90 percent: {id}"
            ),
            Self::InvalidMissingChannel(id) => write!(
                formatter,
                "missing-channel declaration is not in the published channel list: {id}"
            ),
            Self::WrongBaselineCount => write!(
                formatter,
                "dataset must contain exactly one baseline reference file"
            ),
            Self::WrongScenarioCount => write!(
                formatter,
                "dataset must contain exactly 15 fault-scenario files"
            ),
            Self::WrongFamilyCounts(family) => write!(
                formatter,
                "fault-family recording count does not match the audited dataset description: {family:?}"
            ),
            Self::WrongMissingChannelRunCount => write!(
                formatter,
                "exactly five runs must declare at least one of the two published missing channels"
            ),
            Self::WrongAnomalyOnlyFileCount => write!(
                formatter,
                "exactly two injection-nozzle files must be anomaly-only and excluded from event latency"
            ),
            Self::InvalidEventLatencyEligibility(id) => write!(
                formatter,
                "event-latency eligibility contradicts the source label coverage: {id}"
            ),
            Self::InvalidSplitManifest => write!(
                formatter,
                "split manifest must assign every complete file exactly once and contain train/test files"
            ),
            Self::UnknownSplitFile(id) => {
                write!(formatter, "split manifest references unknown file: {id}")
            }
            Self::DuplicateSplitFile(id) => write!(
                formatter,
                "file appears more than once in split manifest: {id}"
            ),
            Self::Serialization => {
                write!(formatter, "failed to serialize canonical dataset evidence")
            }
        }
    }
}

impl std::error::Error for ExternalDatasetError {}

impl MarineExternalDatasetManifest {
    pub fn validate(&self) -> Result<MarineDatasetAuditSummary, ExternalDatasetError> {
        if self.schema_version != MARINE_DATASET_SCHEMA_VERSION
            || self.dataset_doi != MARINE_DATASET_DOI
            || self.dataset_license != "CC-BY-4.0"
        {
            return Err(ExternalDatasetError::WrongDatasetIdentity);
        }
        validate_digest("archive_sha256", &self.archive_sha256)?;
        validate_digest("dataset_index_sha256", &self.dataset_index_sha256)?;
        validate_digest(
            "variable_dictionary_sha256",
            &self.variable_dictionary_sha256,
        )?;
        validate_digest(
            "normalization_script_sha256",
            &self.normalization_script_sha256,
        )?;
        if self.files.len() != MARINE_DATASET_EXPECTED_CSV_FILES {
            return Err(ExternalDatasetError::WrongFileCount);
        }

        let mut file_ids = BTreeSet::new();
        let mut file_paths = BTreeSet::new();
        let mut baseline_count = 0usize;
        let mut scenario_count = 0usize;
        let mut missing_channel_files = 0usize;
        let mut anomaly_only_count = 0usize;
        let mut event_latency_count = 0usize;
        let mut family_counts = BTreeMap::<MarineFaultFamily, usize>::new();
        const ALLOWED_MISSING_CHANNELS: [&str; 2] =
            ["Compressor Filter Loss", "Turbine Back Pressure"];

        for file in &self.files {
            if file.file_id.trim().is_empty() || file.relative_path.trim().is_empty() {
                return Err(ExternalDatasetError::EmptyFileIdentity);
            }
            if !file_ids.insert(file.file_id.as_str()) {
                return Err(ExternalDatasetError::DuplicateFileIdentity(
                    file.file_id.clone(),
                ));
            }
            if !file_paths.insert(file.relative_path.as_str()) {
                return Err(ExternalDatasetError::DuplicateFilePath(
                    file.relative_path.clone(),
                ));
            }
            if !is_sha256_hex(&file.sha256) {
                return Err(ExternalDatasetError::InvalidFileDigest(
                    file.file_id.clone(),
                ));
            }
            if file.row_count == 0 {
                return Err(ExternalDatasetError::EmptyFile(file.file_id.clone()));
            }
            if file.header_rows != MARINE_DATASET_EXPECTED_HEADER_ROWS {
                return Err(ExternalDatasetError::WrongHeaderShape(file.file_id.clone()));
            }
            if file.recorded_column_count != file.schema.expected_column_count() {
                return Err(ExternalDatasetError::WrongColumnCount(file.file_id.clone()));
            }
            if file
                .load_points_percent
                .iter()
                .any(|load| !(30..=90).contains(load))
            {
                return Err(ExternalDatasetError::InvalidLoadPoint(file.file_id.clone()));
            }
            if file
                .missing_channels
                .iter()
                .any(|name| !ALLOWED_MISSING_CHANNELS.contains(&name.as_str()))
            {
                return Err(ExternalDatasetError::InvalidMissingChannel(
                    file.file_id.clone(),
                ));
            }
            if !file.missing_channels.is_empty() {
                missing_channel_files += 1;
            }

            match file.role {
                MarineFileRole::BaselineReference => {
                    baseline_count += 1;
                    if file.schema != MarineCsvSchema::Baseline70
                        || file.fault_family.is_some()
                        || file.anomaly_state_label_present
                        || file.contains_pre_anomaly_samples
                        || file.event_latency_eligible
                        || file.load_points_percent.is_empty()
                    {
                        return Err(ExternalDatasetError::InvalidRoleSemantics(
                            file.file_id.clone(),
                        ));
                    }
                }
                MarineFileRole::FaultScenario => {
                    scenario_count += 1;
                    let Some(family) = file.fault_family else {
                        return Err(ExternalDatasetError::InvalidRoleSemantics(
                            file.file_id.clone(),
                        ));
                    };
                    if file.schema != MarineCsvSchema::Scenario73
                        || !file.anomaly_state_label_present
                        || file.load_points_percent.is_empty()
                    {
                        return Err(ExternalDatasetError::InvalidRoleSemantics(
                            file.file_id.clone(),
                        ));
                    }
                    *family_counts.entry(family).or_default() += 1;
                    if file.contains_pre_anomaly_samples {
                        if !file.event_latency_eligible {
                            return Err(ExternalDatasetError::InvalidEventLatencyEligibility(
                                file.file_id.clone(),
                            ));
                        }
                        event_latency_count += 1;
                    } else {
                        if file.event_latency_eligible
                            || family != MarineFaultFamily::InjectionValveNozzleClogging
                        {
                            return Err(ExternalDatasetError::InvalidEventLatencyEligibility(
                                file.file_id.clone(),
                            ));
                        }
                        anomaly_only_count += 1;
                    }
                }
            }
        }

        if baseline_count != 1 {
            return Err(ExternalDatasetError::WrongBaselineCount);
        }
        if scenario_count != MARINE_DATASET_EXPECTED_SCENARIO_FILES {
            return Err(ExternalDatasetError::WrongScenarioCount);
        }
        let expected_family_counts = [
            (MarineFaultFamily::CompressorAirFilterClogging, 4usize),
            (MarineFaultFamily::AirCoolerFouling, 4usize),
            (MarineFaultFamily::InjectionValveNozzleClogging, 2usize),
            (MarineFaultFamily::CoolingWaterPumpCavitation, 2usize),
            (MarineFaultFamily::TurbineDegradation, 3usize),
        ];
        for (family, expected) in expected_family_counts {
            if family_counts.get(&family).copied().unwrap_or(0) != expected {
                return Err(ExternalDatasetError::WrongFamilyCounts(family));
            }
        }
        if missing_channel_files != 5 {
            return Err(ExternalDatasetError::WrongMissingChannelRunCount);
        }
        if anomaly_only_count != 2 {
            return Err(ExternalDatasetError::WrongAnomalyOnlyFileCount);
        }

        if let Some(split) = &self.split_manifest {
            split.validate_against(&self.files)?;
        }
        let mut ordered_files = self.files.clone();
        ordered_files.sort_by(|left, right| left.file_id.cmp(&right.file_id));
        let file_manifest_sha256 = sha256_serializable(
            b"symtropy-marine-dataset-file-manifest-v1\0",
            &ordered_files,
        )?;
        let split_manifest_sha256 = self
            .split_manifest
            .as_ref()
            .map(MarineFileSplitManifest::digest)
            .transpose()?;

        Ok(MarineDatasetAuditSummary {
            file_count: self.files.len(),
            baseline_file_count: baseline_count,
            scenario_file_count: scenario_count,
            missing_channel_file_count: missing_channel_files,
            anomaly_only_file_count: anomaly_only_count,
            event_latency_eligible_file_count: event_latency_count,
            family_file_counts: family_counts,
            file_manifest_sha256,
            split_manifest_sha256,
        })
    }
}

fn validate_digest(field: &'static str, value: &str) -> Result<(), ExternalDatasetError> {
    if !is_sha256_hex(value) {
        return Err(ExternalDatasetError::InvalidDigest(field));
    }
    Ok(())
}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, ExternalDatasetError> {
    let encoded = serde_json::to_vec(value).map_err(|_| ExternalDatasetError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((encoded.len() as u64).to_le_bytes());
    hasher.update(encoded);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn file(
        index: usize,
        role: MarineFileRole,
        fault_family: Option<MarineFaultFamily>,
        anomaly_only: bool,
        has_missing_channels: bool,
    ) -> MarineDatasetFile {
        let is_baseline = role == MarineFileRole::BaselineReference;
        let schema = if is_baseline {
            MarineCsvSchema::Baseline70
        } else {
            MarineCsvSchema::Scenario73
        };
        let missing_channels = if has_missing_channels {
            BTreeSet::from(["Compressor Filter Loss".to_owned()])
        } else {
            BTreeSet::new()
        };
        MarineDatasetFile {
            file_id: format!("file-{index:02}"),
            relative_path: format!("data/file-{index:02}.csv"),
            sha256: digest(char::from(b"0123456789abcdef"[index % 16])),
            row_count: 1000,
            schema,
            header_rows: 3,
            recorded_column_count: schema.expected_column_count(),
            role,
            fault_family,
            load_points_percent: vec![40, 60],
            anomaly_state_label_present: !is_baseline,
            contains_pre_anomaly_samples: !is_baseline && !anomaly_only,
            event_latency_eligible: !is_baseline && !anomaly_only,
            missing_channels,
        }
    }

    fn manifest() -> MarineExternalDatasetManifest {
        let mut files = vec![file(
            0,
            MarineFileRole::BaselineReference,
            None,
            false,
            false,
        )];
        let families = [
            (MarineFaultFamily::CompressorAirFilterClogging, 4usize),
            (MarineFaultFamily::AirCoolerFouling, 4usize),
            (MarineFaultFamily::InjectionValveNozzleClogging, 2usize),
            (MarineFaultFamily::CoolingWaterPumpCavitation, 2usize),
            (MarineFaultFamily::TurbineDegradation, 3usize),
        ];
        let mut index = 1usize;
        let mut missing_count = 0usize;
        for (family, count) in families {
            for _local_index in 0..count {
                let anomaly_only = family == MarineFaultFamily::InjectionValveNozzleClogging;
                let has_missing = missing_count < 5;
                if has_missing {
                    missing_count += 1;
                }
                files.push(file(
                    index,
                    MarineFileRole::FaultScenario,
                    Some(family),
                    anomaly_only,
                    has_missing,
                ));
                index += 1;
            }
        }
        MarineExternalDatasetManifest {
            schema_version: MARINE_DATASET_SCHEMA_VERSION.into(),
            dataset_doi: MARINE_DATASET_DOI.into(),
            dataset_license: "CC-BY-4.0".into(),
            archive_sha256: digest('a'),
            dataset_index_sha256: digest('b'),
            variable_dictionary_sha256: digest('c'),
            normalization_script_sha256: digest('d'),
            files,
            split_manifest: None,
        }
    }

    #[test]
    fn metadata_protocol_and_file_manifest_schema_are_frozen_separately() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../external-marine-engine-dataset-v0.1.json"))
                .unwrap();
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../external-dataset-file-manifest.schema.json"
        ))
        .unwrap();
        assert_eq!(
            protocol["schema_version"],
            "symtropy-marine-external-dataset-track-v0.1"
        );
        assert_eq!(
            protocol["status"],
            "archive_payload_audited_no_model_evaluation"
        );
        assert_eq!(protocol["dataset"]["doi"], MARINE_DATASET_DOI);
        assert_eq!(protocol["dataset"]["license"], "CC-BY-4.0");
        assert_eq!(
            protocol["dataset"]["zenodo_record"]["file"]["sha256"],
            "3fba7aa0c288ae1bc05383fb54bf67cbde209d61b26e3e811d4bc2f22f97442b"
        );
        assert_eq!(
            protocol["archive_audit_receipt"]["artifact_id"],
            14014574182_u64
        );
        assert_eq!(
            protocol["archive_audit_receipt"]["archive_sha256"],
            protocol["dataset"]["zenodo_record"]["file"]["sha256"]
        );
        assert_eq!(protocol["archive_audit_receipt"]["rust_tests_failed"], 0);
        assert_eq!(protocol["dataset"]["files"]["csv_count"], 16);
        assert_eq!(protocol["dataset"]["files"]["baseline_reference_files"], 1);
        assert_eq!(protocol["dataset"]["files"]["fault_scenario_files"], 15);
        assert_eq!(protocol["fault_scenarios"].as_array().unwrap().len(), 5);
        assert!(
            protocol["research_targets_not_achieved"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| {
                    target["status"] == "aspirational"
                        || target["status"] == "gate_not_performance_target"
                })
        );
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["properties"]["files"]["minItems"], 16);
        assert_eq!(schema["properties"]["files"]["maxItems"], 16);
        assert_eq!(
            schema["$defs"]["file"]["properties"]["header_rows"]["const"],
            3
        );
        assert_eq!(
            schema["$defs"]["file"]["properties"]["recorded_column_count"]["enum"],
            serde_json::json!([70, 73])
        );
    }

    #[test]
    fn valid_file_manifest_replays_expected_inventory_and_missingness() {
        let result = manifest().validate().unwrap();
        assert_eq!(result.file_count, 16);
        assert_eq!(result.baseline_file_count, 1);
        assert_eq!(result.scenario_file_count, 15);
        assert_eq!(result.missing_channel_file_count, 5);
        assert_eq!(result.anomaly_only_file_count, 2);
        assert_eq!(result.event_latency_eligible_file_count, 13);
        assert_eq!(result.family_file_counts.values().sum::<usize>(), 15);
    }

    #[test]
    fn wrong_csv_schema_or_header_count_is_rejected() {
        let mut bad_header = manifest();
        bad_header.files[1].header_rows = 2;
        assert_eq!(
            bad_header.validate(),
            Err(ExternalDatasetError::WrongHeaderShape("file-01".into()))
        );

        let mut bad_columns = manifest();
        bad_columns.files[1].recorded_column_count = 70;
        assert_eq!(
            bad_columns.validate(),
            Err(ExternalDatasetError::WrongColumnCount("file-01".into()))
        );
    }

    #[test]
    fn duplicate_files_or_wrong_fault_family_counts_are_rejected() {
        let mut duplicate = manifest();
        duplicate.files[1].file_id = duplicate.files[0].file_id.clone();
        assert_eq!(
            duplicate.validate(),
            Err(ExternalDatasetError::DuplicateFileIdentity(
                "file-00".into()
            ))
        );

        let mut wrong_family = manifest();
        wrong_family.files[1].fault_family = Some(MarineFaultFamily::TurbineDegradation);
        assert_eq!(
            wrong_family.validate(),
            Err(ExternalDatasetError::WrongFamilyCounts(
                MarineFaultFamily::CompressorAirFilterClogging
            ))
        );
    }

    #[test]
    fn anomaly_only_nozzle_files_cannot_claim_event_latency() {
        let mut invalid = manifest();
        let nozzle_id = {
            let nozzle = invalid
                .files
                .iter_mut()
                .find(|file| {
                    file.fault_family == Some(MarineFaultFamily::InjectionValveNozzleClogging)
                })
                .unwrap();
            nozzle.event_latency_eligible = true;
            nozzle.file_id.clone()
        };
        assert_eq!(
            invalid.validate(),
            Err(ExternalDatasetError::InvalidEventLatencyEligibility(
                nozzle_id
            ))
        );
    }

    #[test]
    fn unsupported_missing_channel_is_rejected() {
        let mut invalid = manifest();
        invalid.files[1]
            .missing_channels
            .insert("EngineSpeed".into());
        assert_eq!(
            invalid.validate(),
            Err(ExternalDatasetError::InvalidMissingChannel(
                "file-01".into()
            ))
        );
    }

    #[test]
    fn file_splits_are_whole_file_disjoint_and_complete() {
        let mut valid = manifest();
        let assignments = valid
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| MarineFileSplitAssignment {
                file_id: file.file_id.clone(),
                split: if index % 4 == 0 {
                    MarineDatasetSplit::Test
                } else {
                    MarineDatasetSplit::Train
                },
            })
            .collect();
        valid.split_manifest = Some(MarineFileSplitManifest {
            schema_version: 1,
            assignments,
        });
        let result = valid.validate().unwrap();
        assert!(result.split_manifest_sha256.is_some());

        let mut duplicate = valid;
        let first_file_id = duplicate.split_manifest.as_ref().unwrap().assignments[0]
            .file_id
            .clone();
        duplicate.split_manifest.as_mut().unwrap().assignments[1].file_id = first_file_id;
        assert_eq!(
            duplicate.validate(),
            Err(ExternalDatasetError::DuplicateSplitFile("file-00".into()))
        );
    }
}
