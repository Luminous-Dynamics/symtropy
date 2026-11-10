//! Deterministic, fail-closed selection among measured render profiles.
//!
//! This policy does not render frames, calculate perceptual metrics, capture GPU
//! telemetry, or preserve scene assets. Those facts must arrive in receipts from
//! the existing benchmark/capture/LOD systems. This crate verifies that the
//! supplied measurements satisfy a task-specific contract and chooses the
//! lowest-cost admissible profile.
//!
//! Hard quality, semantic, critical-asset, evidence, and resource gates are
//! independent. No weighted score lets a fast profile compensate for missing
//! uncertainty metadata or a demoted safety-critical object.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: u32 = 1;
const CLAIM_CEILING: &str =
    "finite-measurement-policy-conformance-only;does-not-render-or-validate-the-underlying-measurements";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyInputV1 {
    pub schema_version: u32,
    pub workload_id: String,
    pub scene_capture_sha256: String,
    pub metric_bundle_sha256: String,
    pub contract: QualityContract,
    pub budget: ResourceBudget,
    pub profiles: Vec<MeasuredProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QualityContract {
    pub task_mode: TaskMode,
    /// Normalized error bounds in [0, 1], defined by the referenced metric bundle.
    pub max_spatial_error: f64,
    pub max_temporal_error: f64,
    pub min_task_legibility: f64,
    pub required_semantic_channels: Vec<SemanticChannel>,
    /// Asset IDs that must remain visible/available at the declared task level.
    pub protected_asset_ids: Vec<String>,
    /// Synthetic-only measurements are useful for exploration, but not a frozen
    /// workload comparison when this flag is true.
    pub require_measured_workload: bool,
    /// Set to zero when no human-preference gate is part of the task contract.
    pub min_panel_evaluators: u32,
    /// Optional lower confidence bound for pairwise preference. This is not a
    /// universal quality metric; it is meaningful only under a pinned protocol.
    pub min_preference_lower_95: Option<f64>,
    /// When panel evidence is required, all profiles must use this same study protocol.
    pub human_panel_protocol_sha256: Option<String>,
    /// When panel evidence is required, all profiles must use this same sample set.
    pub human_panel_sample_set_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceBudget {
    pub max_p95_frame_time_us: u64,
    pub max_p99_frame_time_us: u64,
    pub max_peak_vram_bytes: u64,
    /// When present, every candidate must provide measured GPU energy/frame.
    pub max_gpu_energy_uj_per_frame: Option<f64>,
    /// Largest permitted gap between adjacent GPU power samples.
    /// Must be supplied exactly when an energy budget is requested.
    pub max_energy_sample_gap_us: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnergyMeasurement {
    pub source: EnergyTelemetrySource,
    /// Digest of the original monotonic power-sample series used for integration.
    pub sample_series_sha256: String,
    /// Trapezoid-integrated GPU board energy over the measurement window.
    pub integrated_gpu_energy_uj: f64,
    pub measurement_window_us: u64,
    pub rendered_frame_count: u64,
    pub power_sample_count: u32,
    pub max_sample_gap_us: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnergyTelemetrySource {
    NvmlPowerSamples,
    DcgmProfiling,
    ExternalGpuPowerMeter,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeasuredProfile {
    pub profile_id: String,
    pub task_mode: TaskMode,
    pub scene_capture_sha256: String,
    pub metric_bundle_sha256: String,
    pub measurement_receipt_sha256: String,
    pub evidence_class: EvidenceClass,
    pub preserved_asset_ids: Vec<String>,
    pub semantic_channels: Vec<SemanticChannel>,
    pub spatial_error_upper_bound: f64,
    pub temporal_error_upper_bound: f64,
    pub task_legibility_lower_bound: f64,
    pub frame_time_p95_us: u64,
    pub frame_time_p99_us: u64,
    pub peak_vram_bytes: u64,
    pub energy_measurement: Option<EnergyMeasurement>,
    pub human_preference: Option<HumanPreferenceEvidence>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HumanPreferenceEvidence {
    pub evaluator_count: u32,
    pub pairwise_preference_lower_95: f64,
    pub protocol_sha256: String,
    pub sample_set_sha256: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TaskMode {
    Cinematic,
    EngineeringInspection,
    SafetyInspection,
    Accessibility,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SemanticChannel {
    Units,
    ReferenceFrame,
    Uncertainty,
    Freshness,
    EvidenceClass,
    ContrastMetadata,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    FrozenWorkloadMeasurement,
    SyntheticOnly,
    Unqualified,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    SceneCaptureMismatch,
    MetricBundleMismatch,
    TaskModeMismatch,
    SyntheticOnlyEvidence,
    UnqualifiedEvidence,
    ProtectedAssetMissing,
    SemanticChannelMissing,
    SpatialErrorExceeded,
    TemporalErrorExceeded,
    TaskLegibilityBelowMinimum,
    P95FrameTimeBudgetExceeded,
    P99FrameTimeBudgetExceeded,
    VramBudgetExceeded,
    EnergyMeasurementMissing,
    EnergySamplesInsufficient,
    EnergySampleGapExceeded,
    EnergyBudgetExceeded,
    HumanPanelMissing,
    HumanPanelTooSmall,
    PreferenceLowerBoundNotMet,
    HumanPanelProtocolMismatch,
    HumanPanelSampleSetMismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProfileEvaluation {
    pub profile_id: String,
    pub admissible: bool,
    pub rejection_reasons: Vec<RejectionReason>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PolicyDecision {
    Selected {
        profile_id: String,
        measurement_receipt_sha256: String,
    },
    Abstain {
        reason: AbstentionReason,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AbstentionReason {
    NoProfiles,
    NoAdmissibleProfile,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyReceiptV1 {
    pub schema_version: u32,
    pub input_sha256: String,
    pub workload_id: String,
    pub decision: PolicyDecision,
    pub evaluations: Vec<ProfileEvaluation>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    UnsupportedSchema(u32),
    EmptyField(&'static str),
    InvalidSha256(&'static str),
    NonFiniteOrOutOfRange(&'static str),
    NonFiniteOrNegative(&'static str),
    ZeroResourceBudget(&'static str),
    DuplicateProfileId(String),
    DuplicateProtectedAsset(String),
    DuplicatePreservedAsset(String),
    DuplicateSemanticChannel(&'static str),
    MissingContractSemanticChannel(SemanticChannel),
    Serialization(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(v) => write!(f, "unsupported schema version: {v}"),
            Self::EmptyField(name) => write!(f, "required field is empty: {name}"),
            Self::InvalidSha256(name) => write!(f, "invalid SHA-256 identity: {name}"),
            Self::NonFiniteOrOutOfRange(name) => {
                write!(f, "value must be finite and in [0, 1]: {name}")
            }
            Self::NonFiniteOrNegative(name) => {
                write!(f, "value must be finite and non-negative: {name}")
            }
            Self::ZeroResourceBudget(name) => write!(f, "resource budget must be non-zero: {name}"),
            Self::DuplicateProfileId(id) => write!(f, "duplicate profile id: {id}"),
            Self::DuplicateProtectedAsset(id) => write!(f, "duplicate protected asset id: {id}"),
            Self::DuplicatePreservedAsset(id) => write!(f, "duplicate preserved asset id: {id}"),
            Self::DuplicateSemanticChannel(name) => {
                write!(f, "duplicate semantic channel: {name}")
            }
            Self::MissingContractSemanticChannel(channel) => {
                write!(f, "task contract is missing mandatory semantic channel: {channel:?}")
            }
            Self::Serialization(message) => write!(f, "serialization failure: {message}"),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Evaluate a finite profile set and produce a deterministic receipt.
///
/// If an energy cap is specified, GPU energy is a required measurement and
/// selection is lexicographic by energy, p95 frame time, peak VRAM, then ID.
/// Without an energy cap, selection is lexicographic by p95, peak VRAM, then ID.
/// This policy deliberately does not invent missing power/energy values.
pub fn evaluate(input: &PolicyInputV1) -> Result<PolicyReceiptV1, PolicyError> {
    validate_input(input)?;
    let canonical = canonicalize(input);
    let input_sha256 = canonical_sha256(&canonical)?;
    let mut evaluations = Vec::with_capacity(canonical.profiles.len());
    let mut admissible: Vec<&MeasuredProfile> = Vec::new();

    for profile in &canonical.profiles {
        let rejection_reasons = rejection_reasons(&canonical, profile);
        let accepted = rejection_reasons.is_empty();
        if accepted {
            admissible.push(profile);
        }
        evaluations.push(ProfileEvaluation {
            profile_id: profile.profile_id.clone(),
            admissible: accepted,
            rejection_reasons,
        });
    }

    let selected = admissible.into_iter().min_by(|a, b| {
        if canonical.budget.max_gpu_energy_uj_per_frame.is_some() {
            energy_uj_per_frame(a)
                .unwrap_or(f64::INFINITY)
                .total_cmp(&energy_uj_per_frame(b).unwrap_or(f64::INFINITY))
                .then_with(|| a.frame_time_p95_us.cmp(&b.frame_time_p95_us))
                .then_with(|| a.peak_vram_bytes.cmp(&b.peak_vram_bytes))
                .then_with(|| a.profile_id.cmp(&b.profile_id))
        } else {
            a.frame_time_p95_us
                .cmp(&b.frame_time_p95_us)
                .then_with(|| a.peak_vram_bytes.cmp(&b.peak_vram_bytes))
                .then_with(|| a.profile_id.cmp(&b.profile_id))
        }
    });

    let decision = selected
        .map(|profile| PolicyDecision::Selected {
            profile_id: profile.profile_id.clone(),
            measurement_receipt_sha256: profile.measurement_receipt_sha256.clone(),
        })
        .unwrap_or_else(|| PolicyDecision::Abstain {
            reason: if canonical.profiles.is_empty() {
                AbstentionReason::NoProfiles
            } else {
                AbstentionReason::NoAdmissibleProfile
            },
        });

    Ok(PolicyReceiptV1 {
        schema_version: SCHEMA_VERSION,
        input_sha256,
        workload_id: canonical.workload_id,
        decision,
        evaluations,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    })
}

fn energy_uj_per_frame(profile: &MeasuredProfile) -> Option<f64> {
    let measurement = profile.energy_measurement.as_ref()?;
    if measurement.rendered_frame_count == 0 {
        return None;
    }
    Some(measurement.integrated_gpu_energy_uj / measurement.rendered_frame_count as f64)
}

fn rejection_reasons(
    input: &PolicyInputV1,
    profile: &MeasuredProfile,
) -> Vec<RejectionReason> {
    let mut reasons = Vec::new();

    if profile.scene_capture_sha256 != input.scene_capture_sha256 {
        reasons.push(RejectionReason::SceneCaptureMismatch);
    }
    if profile.metric_bundle_sha256 != input.metric_bundle_sha256 {
        reasons.push(RejectionReason::MetricBundleMismatch);
    }
    if profile.task_mode != input.contract.task_mode {
        reasons.push(RejectionReason::TaskModeMismatch);
    }

    match profile.evidence_class {
        EvidenceClass::FrozenWorkloadMeasurement => {}
        EvidenceClass::SyntheticOnly if input.contract.require_measured_workload => {
            reasons.push(RejectionReason::SyntheticOnlyEvidence)
        }
        EvidenceClass::SyntheticOnly => {}
        EvidenceClass::Unqualified => reasons.push(RejectionReason::UnqualifiedEvidence),
    }

    for asset_id in &input.contract.protected_asset_ids {
        if !profile.preserved_asset_ids.contains(asset_id) {
            reasons.push(RejectionReason::ProtectedAssetMissing);
            break;
        }
    }
    for channel in &input.contract.required_semantic_channels {
        if !profile.semantic_channels.contains(channel) {
            reasons.push(RejectionReason::SemanticChannelMissing);
            break;
        }
    }

    if profile.spatial_error_upper_bound > input.contract.max_spatial_error {
        reasons.push(RejectionReason::SpatialErrorExceeded);
    }
    if profile.temporal_error_upper_bound > input.contract.max_temporal_error {
        reasons.push(RejectionReason::TemporalErrorExceeded);
    }
    if profile.task_legibility_lower_bound < input.contract.min_task_legibility {
        reasons.push(RejectionReason::TaskLegibilityBelowMinimum);
    }
    if profile.frame_time_p95_us > input.budget.max_p95_frame_time_us {
        reasons.push(RejectionReason::P95FrameTimeBudgetExceeded);
    }
    if profile.frame_time_p99_us > input.budget.max_p99_frame_time_us {
        reasons.push(RejectionReason::P99FrameTimeBudgetExceeded);
    }
    if profile.peak_vram_bytes > input.budget.max_peak_vram_bytes {
        reasons.push(RejectionReason::VramBudgetExceeded);
    }

    if let Some(max_energy) = input.budget.max_gpu_energy_uj_per_frame {
        match &profile.energy_measurement {
            None => reasons.push(RejectionReason::EnergyMeasurementMissing),
            Some(measurement) => {
                if measurement.power_sample_count < 2 {
                    reasons.push(RejectionReason::EnergySamplesInsufficient);
                }
                if measurement.max_sample_gap_us
                    > input.budget.max_energy_sample_gap_us.unwrap_or(0)
                    || measurement.measurement_window_us < measurement.max_sample_gap_us
                {
                    reasons.push(RejectionReason::EnergySampleGapExceeded);
                }
                if energy_uj_per_frame(profile).is_some_and(|energy| energy > max_energy) {
                    reasons.push(RejectionReason::EnergyBudgetExceeded);
                }
            }
        }
    }

    let panel_required = input.contract.min_panel_evaluators > 0
        || input.contract.min_preference_lower_95.is_some();
    if panel_required {
        match &profile.human_preference {
            None => reasons.push(RejectionReason::HumanPanelMissing),
            Some(panel) => {
                if panel.evaluator_count < input.contract.min_panel_evaluators {
                    reasons.push(RejectionReason::HumanPanelTooSmall);
                }
                if let Some(minimum) = input.contract.min_preference_lower_95 {
                    if panel.pairwise_preference_lower_95 < minimum {
                        reasons.push(RejectionReason::PreferenceLowerBoundNotMet);
                    }
                }
                if input
                    .contract
                    .human_panel_protocol_sha256
                    .as_ref()
                    .is_some_and(|expected| expected != &panel.protocol_sha256)
                {
                    reasons.push(RejectionReason::HumanPanelProtocolMismatch);
                }
                if input
                    .contract
                    .human_panel_sample_set_sha256
                    .as_ref()
                    .is_some_and(|expected| expected != &panel.sample_set_sha256)
                {
                    reasons.push(RejectionReason::HumanPanelSampleSetMismatch);
                }
            }
        }
    }

    reasons
}

fn validate_input(input: &PolicyInputV1) -> Result<(), PolicyError> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(PolicyError::UnsupportedSchema(input.schema_version));
    }
    validate_nonempty(&input.workload_id, "workload_id")?;
    validate_sha256(&input.scene_capture_sha256, "scene_capture_sha256")?;
    validate_sha256(&input.metric_bundle_sha256, "metric_bundle_sha256")?;
    validate_unit_interval(input.contract.max_spatial_error, "contract.max_spatial_error")?;
    validate_unit_interval(input.contract.max_temporal_error, "contract.max_temporal_error")?;
    validate_unit_interval(input.contract.min_task_legibility, "contract.min_task_legibility")?;
    if let Some(value) = input.contract.min_preference_lower_95 {
        validate_unit_interval(value, "contract.min_preference_lower_95")?;
    }
    if input.contract.min_panel_evaluators > 0
        || input.contract.min_preference_lower_95.is_some()
    {
        let protocol = input
            .contract
            .human_panel_protocol_sha256
            .as_deref()
            .ok_or(PolicyError::EmptyField("contract.human_panel_protocol_sha256"))?;
        let sample_set = input
            .contract
            .human_panel_sample_set_sha256
            .as_deref()
            .ok_or(PolicyError::EmptyField("contract.human_panel_sample_set_sha256"))?;
        validate_sha256(protocol, "contract.human_panel_protocol_sha256")?;
        validate_sha256(sample_set, "contract.human_panel_sample_set_sha256")?;
    } else if input.contract.human_panel_protocol_sha256.is_some()
        || input.contract.human_panel_sample_set_sha256.is_some()
    {
        return Err(PolicyError::EmptyField(
            "contract.human_panel_protocol_sha256 and sample_set_sha256 must be paired with a panel gate",
        ));
    }
    if input.budget.max_p95_frame_time_us == 0 {
        return Err(PolicyError::ZeroResourceBudget("max_p95_frame_time_us"));
    }
    if input.budget.max_p99_frame_time_us == 0 {
        return Err(PolicyError::ZeroResourceBudget("max_p99_frame_time_us"));
    }
    if input.budget.max_peak_vram_bytes == 0 {
        return Err(PolicyError::ZeroResourceBudget("max_peak_vram_bytes"));
    }
    match (
        input.budget.max_gpu_energy_uj_per_frame,
        input.budget.max_energy_sample_gap_us,
    ) {
        (Some(value), Some(max_gap)) => {
            validate_nonnegative_finite(value, "budget.max_gpu_energy_uj_per_frame")?;
            if max_gap == 0 {
                return Err(PolicyError::ZeroResourceBudget("budget.max_energy_sample_gap_us"));
            }
        }
        (Some(_), None) => {
            return Err(PolicyError::EmptyField(
                "budget.max_energy_sample_gap_us required with energy budget",
            ));
        }
        (None, Some(_)) => {
            return Err(PolicyError::EmptyField(
                "budget.max_energy_sample_gap_us must be absent without energy budget",
            ));
        }
        (None, None) => {}
    }

    let mut protected = BTreeSet::new();
    for asset_id in &input.contract.protected_asset_ids {
        validate_nonempty(asset_id, "contract.protected_asset_ids[]")?;
        if !protected.insert(asset_id.clone()) {
            return Err(PolicyError::DuplicateProtectedAsset(asset_id.clone()));
        }
    }
    validate_unique_channels(
        &input.contract.required_semantic_channels,
        "contract.required_semantic_channels",
    )?;
    validate_mode_contract(&input.contract)?;

    let mut profile_ids = BTreeSet::new();
    for profile in &input.profiles {
        validate_nonempty(&profile.profile_id, "profile.profile_id")?;
        validate_sha256(&profile.scene_capture_sha256, "profile.scene_capture_sha256")?;
        validate_sha256(&profile.metric_bundle_sha256, "profile.metric_bundle_sha256")?;
        validate_sha256(
            &profile.measurement_receipt_sha256,
            "profile.measurement_receipt_sha256",
        )?;
        validate_unit_interval(profile.spatial_error_upper_bound, "profile.spatial_error_upper_bound")?;
        validate_unit_interval(profile.temporal_error_upper_bound, "profile.temporal_error_upper_bound")?;
        validate_unit_interval(profile.task_legibility_lower_bound, "profile.task_legibility_lower_bound")?;
        if profile.frame_time_p95_us == 0 {
            return Err(PolicyError::ZeroResourceBudget("profile.frame_time_p95_us"));
        }
        if profile.frame_time_p99_us == 0 {
            return Err(PolicyError::ZeroResourceBudget("profile.frame_time_p99_us"));
        }
        if profile.peak_vram_bytes == 0 {
            return Err(PolicyError::ZeroResourceBudget("profile.peak_vram_bytes"));
        }
        if let Some(measurement) = &profile.energy_measurement {
            validate_sha256(
                &measurement.sample_series_sha256,
                "profile.energy_measurement.sample_series_sha256",
            )?;
            validate_nonnegative_finite(
                measurement.integrated_gpu_energy_uj,
                "profile.energy_measurement.integrated_gpu_energy_uj",
            )?;
            if measurement.measurement_window_us == 0 {
                return Err(PolicyError::ZeroResourceBudget(
                    "profile.energy_measurement.measurement_window_us",
                ));
            }
            if measurement.rendered_frame_count == 0 {
                return Err(PolicyError::ZeroResourceBudget(
                    "profile.energy_measurement.rendered_frame_count",
                ));
            }
            if measurement.max_sample_gap_us == 0 {
                return Err(PolicyError::ZeroResourceBudget(
                    "profile.energy_measurement.max_sample_gap_us",
                ));
            }
        }
        if !profile_ids.insert(profile.profile_id.clone()) {
            return Err(PolicyError::DuplicateProfileId(profile.profile_id.clone()));
        }

        let mut assets = BTreeSet::new();
        for asset_id in &profile.preserved_asset_ids {
            validate_nonempty(asset_id, "profile.preserved_asset_ids[]")?;
            if !assets.insert(asset_id.clone()) {
                return Err(PolicyError::DuplicatePreservedAsset(asset_id.clone()));
            }
        }
        validate_unique_channels(&profile.semantic_channels, "profile.semantic_channels")?;

        if let Some(panel) = &profile.human_preference {
            if panel.evaluator_count == 0 {
                return Err(PolicyError::ZeroResourceBudget("human_preference.evaluator_count"));
            }
            validate_unit_interval(
                panel.pairwise_preference_lower_95,
                "human_preference.pairwise_preference_lower_95",
            )?;
            validate_sha256(&panel.protocol_sha256, "human_preference.protocol_sha256")?;
            validate_sha256(&panel.sample_set_sha256, "human_preference.sample_set_sha256")?;
        }
    }
    Ok(())
}

fn validate_mode_contract(contract: &QualityContract) -> Result<(), PolicyError> {
    let mandatory: &[SemanticChannel] = match contract.task_mode {
        TaskMode::Cinematic => &[],
        TaskMode::EngineeringInspection | TaskMode::SafetyInspection => &[
            SemanticChannel::Units,
            SemanticChannel::ReferenceFrame,
            SemanticChannel::Uncertainty,
            SemanticChannel::Freshness,
            SemanticChannel::EvidenceClass,
        ],
        TaskMode::Accessibility => &[SemanticChannel::ContrastMetadata],
    };

    for channel in mandatory {
        if !contract.required_semantic_channels.contains(channel) {
            return Err(PolicyError::MissingContractSemanticChannel(*channel));
        }
    }
    if contract.task_mode == TaskMode::SafetyInspection
        && contract.protected_asset_ids.is_empty()
    {
        return Err(PolicyError::EmptyField(
            "contract.protected_asset_ids required for safety_inspection",
        ));
    }
    Ok(())
}

fn validate_unique_channels(
    channels: &[SemanticChannel],
    field: &'static str,
) -> Result<(), PolicyError> {
    let mut seen = BTreeSet::new();
    for channel in channels {
        if !seen.insert(*channel) {
            return Err(PolicyError::DuplicateSemanticChannel(field));
        }
    }
    Ok(())
}

fn canonicalize(input: &PolicyInputV1) -> PolicyInputV1 {
    let mut canonical = input.clone();
    canonical.contract.required_semantic_channels.sort();
    canonical.contract.protected_asset_ids.sort();
    canonical.profiles.sort_by(|a, b| a.profile_id.cmp(&b.profile_id));
    for profile in &mut canonical.profiles {
        profile.preserved_asset_ids.sort();
        profile.semantic_channels.sort();
    }
    canonical
}

fn canonical_sha256(input: &PolicyInputV1) -> Result<String, PolicyError> {
    let bytes = serde_json::to_vec(input)
        .map_err(|error| PolicyError::Serialization(error.to_string()))?;
    Ok(hex_sha256(&bytes))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn validate_nonempty(value: &str, field: &'static str) -> Result<(), PolicyError> {
    if value.trim().is_empty() {
        Err(PolicyError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_sha256(value: &str, field: &'static str) -> Result<(), PolicyError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PolicyError::InvalidSha256(field));
    }
    Ok(())
}

fn validate_unit_interval(value: f64, field: &'static str) -> Result<(), PolicyError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        Err(PolicyError::NonFiniteOrOutOfRange(field))
    } else {
        Ok(())
    }
}

fn validate_nonnegative_finite(value: f64, field: &'static str) -> Result<(), PolicyError> {
    if !value.is_finite() || value < 0.0 {
        Err(PolicyError::NonFiniteOrNegative(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const H2: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const H3: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const H4: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn profile(id: &str, energy: Option<f64>, p95: u64) -> MeasuredProfile {
        MeasuredProfile {
            profile_id: id.to_owned(),
            task_mode: TaskMode::EngineeringInspection,
            scene_capture_sha256: H1.to_owned(),
            metric_bundle_sha256: H2.to_owned(),
            measurement_receipt_sha256: H3.to_owned(),
            evidence_class: EvidenceClass::FrozenWorkloadMeasurement,
            preserved_asset_ids: vec!["asset-critical-01".to_owned()],
            semantic_channels: vec![
                SemanticChannel::Units,
                SemanticChannel::ReferenceFrame,
                SemanticChannel::Uncertainty,
                SemanticChannel::Freshness,
                SemanticChannel::EvidenceClass,
            ],
            spatial_error_upper_bound: 0.04,
            temporal_error_upper_bound: 0.05,
            task_legibility_lower_bound: 0.95,
            frame_time_p95_us: p95,
            frame_time_p99_us: p95 + 2_000,
            peak_vram_bytes: 4_000_000_000,
            energy_measurement: energy.map(|energy_per_frame| EnergyMeasurement {
                source: EnergyTelemetrySource::NvmlPowerSamples,
                sample_series_sha256: H4.to_owned(),
                integrated_gpu_energy_uj: energy_per_frame * 1_000.0,
                measurement_window_us: 16_000_000,
                rendered_frame_count: 1_000,
                power_sample_count: 17,
                max_sample_gap_us: 1_000_000,
            }),
            human_preference: None,
        }
    }

    fn input() -> PolicyInputV1 {
        PolicyInputV1 {
            schema_version: SCHEMA_VERSION,
            workload_id: "city-block-frozen-camera-rail-01".to_owned(),
            scene_capture_sha256: H1.to_owned(),
            metric_bundle_sha256: H2.to_owned(),
            contract: QualityContract {
                task_mode: TaskMode::EngineeringInspection,
                max_spatial_error: 0.1,
                max_temporal_error: 0.1,
                min_task_legibility: 0.8,
                required_semantic_channels: vec![
                    SemanticChannel::Units,
                    SemanticChannel::ReferenceFrame,
                    SemanticChannel::Uncertainty,
                    SemanticChannel::Freshness,
                    SemanticChannel::EvidenceClass,
                ],
                protected_asset_ids: vec!["asset-critical-01".to_owned()],
                require_measured_workload: true,
                min_panel_evaluators: 0,
                min_preference_lower_95: None,
                human_panel_protocol_sha256: None,
                human_panel_sample_set_sha256: None,
            },
            budget: ResourceBudget {
                max_p95_frame_time_us: 16_700,
                max_p99_frame_time_us: 25_000,
                max_peak_vram_bytes: 6_000_000_000,
                max_gpu_energy_uj_per_frame: Some(8_000.0),
                max_energy_sample_gap_us: Some(1_250_000),
            },
            profiles: vec![
                profile("high-quality", Some(7_500.0), 14_000),
                profile("balanced", Some(5_000.0), 12_000),
            ],
        }
    }

    #[test]
    fn selects_lowest_energy_profile_inside_every_hard_gate() {
        let receipt = evaluate(&input()).expect("valid input");
        assert_eq!(
            receipt.decision,
            PolicyDecision::Selected {
                profile_id: "balanced".to_owned(),
                measurement_receipt_sha256: H3.to_owned(),
            }
        );
    }

    #[test]
    fn candidate_order_and_set_order_do_not_change_canonical_receipt() {
        let forward = input();
        let mut shuffled = forward.clone();
        shuffled.profiles.reverse();
        shuffled.contract.protected_asset_ids.reverse();
        shuffled.contract.required_semantic_channels.reverse();
        for profile in &mut shuffled.profiles {
            profile.semantic_channels.reverse();
            profile.preserved_asset_ids.reverse();
        }
        let a = evaluate(&forward).expect("forward");
        let b = evaluate(&shuffled).expect("shuffled");
        assert_eq!(a.input_sha256, b.input_sha256);
        assert_eq!(a.decision, b.decision);
        assert_eq!(a.evaluations, b.evaluations);
    }

    #[test]
    fn protected_asset_cannot_be_demoted_even_when_resources_pass() {
        let mut data = input();
        data.profiles[0].preserved_asset_ids.clear();
        let receipt = evaluate(&data).expect("valid measurement set");
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.profile_id == "high-quality"
                && entry.rejection_reasons.contains(&RejectionReason::ProtectedAssetMissing)
        }));
    }

    #[test]
    fn engineering_semantics_are_hard_requirements() {
        let mut data = input();
        data.profiles[0].semantic_channels.retain(|channel| *channel != SemanticChannel::Uncertainty);
        let receipt = evaluate(&data).expect("valid measurement set");
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.profile_id == "high-quality"
                && entry.rejection_reasons.contains(&RejectionReason::SemanticChannelMissing)
        }));
    }

    #[test]
    fn synthetic_only_measurements_are_rejected_for_frozen_workloads() {
        let mut data = input();
        data.profiles[0].evidence_class = EvidenceClass::SyntheticOnly;
        let receipt = evaluate(&data).expect("valid synthetic record");
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.profile_id == "high-quality"
                && entry.rejection_reasons.contains(&RejectionReason::SyntheticOnlyEvidence)
        }));
    }

    #[test]
    fn synthetic_exploration_can_be_allowed_only_by_explicit_contract() {
        let mut data = input();
        data.profiles.truncate(1);
        data.profiles[0].evidence_class = EvidenceClass::SyntheticOnly;
        data.contract.require_measured_workload = false;
        let receipt = evaluate(&data).expect("valid exploratory record");
        assert!(matches!(
            receipt.decision,
            PolicyDecision::Selected { ref profile_id, .. } if profile_id == "high-quality"
        ));
    }

    #[test]
    fn mismatched_scene_and_metric_receipts_are_rejected() {
        let mut data = input();
        data.profiles[0].scene_capture_sha256 = H4.to_owned();
        data.profiles[1].metric_bundle_sha256 = H4.to_owned();
        let receipt = evaluate(&data).expect("well-formed identities");
        assert!(receipt.evaluations.iter().all(|entry| !entry.admissible));
        assert_eq!(
            receipt.decision,
            PolicyDecision::Abstain { reason: AbstentionReason::NoAdmissibleProfile }
        );
    }

    #[test]
    fn energy_is_not_used_as_a_hidden_objective_when_not_requested() {
        let mut data = input();
        data.budget.max_gpu_energy_uj_per_frame = None;
        data.profiles[0].frame_time_p95_us = 9_000;
        data.profiles[1].frame_time_p95_us = 12_000;
        data.profiles[0].energy_measurement = Some(EnergyMeasurement {
            source: EnergyTelemetrySource::NvmlPowerSamples,
            sample_series_sha256: H4.to_owned(),
            integrated_gpu_energy_uj: 7_500_000.0,
            measurement_window_us: 16_000_000,
            rendered_frame_count: 1_000,
            power_sample_count: 17,
            max_sample_gap_us: 1_000_000,
        });
        data.profiles[1].energy_measurement = Some(EnergyMeasurement {
            source: EnergyTelemetrySource::NvmlPowerSamples,
            sample_series_sha256: H3.to_owned(),
            integrated_gpu_energy_uj: 5_000_000.0,
            measurement_window_us: 16_000_000,
            rendered_frame_count: 1_000,
            power_sample_count: 17,
            max_sample_gap_us: 1_000_000,
        });
        let receipt = evaluate(&data).expect("no energy cap requested");
        assert_eq!(
            receipt.decision,
            PolicyDecision::Selected {
                profile_id: "high-quality".to_owned(),
                measurement_receipt_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned(),
            }
        );
    }

    #[test]
    fn requested_energy_measurement_cannot_be_missing() {
        let mut data = input();
        data.profiles[0].energy_measurement = None;
        let receipt = evaluate(&data).expect("valid record with missing optional energy");
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.profile_id == "high-quality"
                && entry.rejection_reasons.contains(&RejectionReason::EnergyMeasurementMissing)
        }));
    }

    #[test]
    fn requested_energy_budget_is_enforced() {
        let mut data = input();
        data.profiles[0].energy_measurement = Some(EnergyMeasurement {
            source: EnergyTelemetrySource::NvmlPowerSamples,
            sample_series_sha256: H4.to_owned(),
            integrated_gpu_energy_uj: 9_000_000.0,
            measurement_window_us: 16_000_000,
            rendered_frame_count: 1_000,
            power_sample_count: 17,
            max_sample_gap_us: 1_000_000,
        });
        let receipt = evaluate(&data).expect("valid record");
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.profile_id == "high-quality"
                && entry.rejection_reasons.contains(&RejectionReason::EnergyBudgetExceeded)
        }));
    }

    #[test]
    fn insufficient_human_panel_is_rejected() {
        let mut data = input();
        data.contract.min_panel_evaluators = 20;
        data.contract.min_preference_lower_95 = Some(0.55);
        data.contract.human_panel_protocol_sha256 = Some(H3.to_owned());
        data.contract.human_panel_sample_set_sha256 = Some(H4.to_owned());
        let receipt = evaluate(&data).expect("profiles have no panel");
        assert!(receipt.evaluations.iter().all(|entry| {
            entry.rejection_reasons.contains(&RejectionReason::HumanPanelMissing)
        }));
    }

    #[test]
    fn panel_protocol_and_sample_set_must_match_contract() {
        let mut data = input();
        data.contract.min_panel_evaluators = 20;
        data.contract.human_panel_protocol_sha256 = Some(H1.to_owned());
        data.contract.human_panel_sample_set_sha256 = Some(H2.to_owned());
        for candidate in &mut data.profiles {
            candidate.human_preference = Some(HumanPreferenceEvidence {
                evaluator_count: 24,
                pairwise_preference_lower_95: 0.6,
                protocol_sha256: H3.to_owned(),
                sample_set_sha256: H4.to_owned(),
            });
        }
        let receipt = evaluate(&data).expect("well-formed but mismatched study provenance");
        assert!(receipt.evaluations.iter().all(|entry| {
            entry.rejection_reasons.contains(&RejectionReason::HumanPanelProtocolMismatch)
                && entry.rejection_reasons.contains(&RejectionReason::HumanPanelSampleSetMismatch)
        }));
    }

    #[test]
    fn panel_must_meet_both_count_and_confidence_bound() {
        let mut data = input();
        data.contract.min_panel_evaluators = 20;
        data.contract.min_preference_lower_95 = Some(0.55);
        data.contract.human_panel_protocol_sha256 = Some(H3.to_owned());
        data.contract.human_panel_sample_set_sha256 = Some(H4.to_owned());
        for candidate in &mut data.profiles {
            candidate.human_preference = Some(HumanPreferenceEvidence {
                evaluator_count: 24,
                pairwise_preference_lower_95: 0.51,
                protocol_sha256: H3.to_owned(),
                sample_set_sha256: H4.to_owned(),
            });
        }
        let receipt = evaluate(&data).expect("valid panel receipt");
        assert!(receipt.evaluations.iter().all(|entry| {
            entry.rejection_reasons.contains(&RejectionReason::PreferenceLowerBoundNotMet)
        }));
    }

    #[test]
    fn temporal_quality_cannot_be_traded_for_frame_time() {
        let mut data = input();
        data.profiles[0].temporal_error_upper_bound = 0.2;
        data.profiles[0].frame_time_p95_us = 100;
        let receipt = evaluate(&data).expect("valid record");
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.profile_id == "high-quality"
                && entry.rejection_reasons.contains(&RejectionReason::TemporalErrorExceeded)
        }));
    }

    #[test]
    fn p95_p99_and_vram_are_independent_hard_budgets() {
        let mut data = input();
        data.profiles[0].frame_time_p99_us = 30_000;
        data.profiles[1].peak_vram_bytes = 7_000_000_000;
        let receipt = evaluate(&data).expect("valid record");
        let high_quality = receipt.evaluations.iter().find(|entry| entry.profile_id == "high-quality").expect("high profile");
        let balanced = receipt.evaluations.iter().find(|entry| entry.profile_id == "balanced").expect("balanced profile");
        assert!(high_quality.rejection_reasons.contains(&RejectionReason::P99FrameTimeBudgetExceeded));
        assert!(balanced.rejection_reasons.contains(&RejectionReason::VramBudgetExceeded));
    }

    #[test]
    fn human_preference_is_optional_when_not_requested() {
        let receipt = evaluate(&input()).expect("no panel required");
        assert!(receipt.evaluations.iter().all(|entry| entry.rejection_reasons.is_empty()));
    }

    #[test]
    fn profile_from_a_different_task_mode_is_rejected() {
        let mut data = input();
        data.profiles[0].task_mode = TaskMode::Accessibility;
        let receipt = evaluate(&data).expect("well-formed but mismatched profile");
        let high_quality = receipt
            .evaluations
            .iter()
            .find(|entry| entry.profile_id == "high-quality")
            .expect("high quality evaluation");
        assert!(high_quality.rejection_reasons.contains(&RejectionReason::TaskModeMismatch));
    }

    #[test]
    fn engineering_contract_cannot_omit_uncertainty_semantics() {
        let mut data = input();
        data.contract
            .required_semantic_channels
            .retain(|channel| *channel != SemanticChannel::Uncertainty);
        assert_eq!(
            evaluate(&data).unwrap_err(),
            PolicyError::MissingContractSemanticChannel(SemanticChannel::Uncertainty)
        );
    }

    #[test]
    fn safety_contract_requires_at_least_one_protected_asset() {
        let mut data = input();
        data.contract.task_mode = TaskMode::SafetyInspection;
        data.contract.protected_asset_ids.clear();
        for profile in &mut data.profiles {
            profile.task_mode = TaskMode::SafetyInspection;
        }
        assert_eq!(
            evaluate(&data).unwrap_err(),
            PolicyError::EmptyField("contract.protected_asset_ids required for safety_inspection")
        );
    }

    #[test]
    fn accessibility_contract_requires_contrast_metadata() {
        let mut data = input();
        data.contract.task_mode = TaskMode::Accessibility;
        data.contract.required_semantic_channels
            .retain(|channel| *channel != SemanticChannel::ContrastMetadata);
        for profile in &mut data.profiles {
            profile.task_mode = TaskMode::Accessibility;
        }
        assert_eq!(
            evaluate(&data).unwrap_err(),
            PolicyError::MissingContractSemanticChannel(SemanticChannel::ContrastMetadata)
        );
    }

    #[test]
    fn duplicate_profiles_fail_closed() {
        let mut data = input();
        data.profiles[1].profile_id = data.profiles[0].profile_id.clone();
        assert!(matches!(evaluate(&data), Err(PolicyError::DuplicateProfileId(_))));
    }

    #[test]
    fn duplicate_preserved_asset_ids_fail_closed() {
        let mut data = input();
        data.profiles[0].preserved_asset_ids.push("asset-critical-01".to_owned());
        assert!(matches!(evaluate(&data), Err(PolicyError::DuplicatePreservedAsset(_))));
    }

    #[test]
    fn unqualified_measurements_are_never_selected() {
        let mut data = input();
        data.profiles[0].evidence_class = EvidenceClass::Unqualified;
        let receipt = evaluate(&data).expect("well-formed input");
        assert!(receipt.evaluations[0].rejection_reasons.contains(&RejectionReason::UnqualifiedEvidence));
    }

    #[test]
    fn no_admissible_profile_abstains_without_relaxing_limits() {
        let mut data = input();
        data.contract.max_temporal_error = 0.001;
        let receipt = evaluate(&data).expect("well-formed input");
        assert_eq!(
            receipt.decision,
            PolicyDecision::Abstain { reason: AbstentionReason::NoAdmissibleProfile }
        );
    }

    #[test]
    fn malformed_digest_fails_closed() {
        let mut data = input();
        data.profiles[0].measurement_receipt_sha256 = "not-a-digest".to_owned();
        assert_eq!(evaluate(&data).unwrap_err(), PolicyError::InvalidSha256("profile.measurement_receipt_sha256"));
    }

    #[test]
    fn non_finite_metrics_fail_closed() {
        let mut data = input();
        data.profiles[0].temporal_error_upper_bound = f64::NAN;
        assert_eq!(
            evaluate(&data).unwrap_err(),
            PolicyError::NonFiniteOrOutOfRange("profile.temporal_error_upper_bound")
        );
    }
}
