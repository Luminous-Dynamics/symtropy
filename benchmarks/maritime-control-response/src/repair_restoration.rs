#![forbid(unsafe_code)]

//! Read-only assessment of repair-to-restoration evidence under matched duty.
//!
//! This module distinguishes a repair record from an independently observed
//! restoration of physical capability. It checks declared identity, chronology,
//! matched operating conditions, capability envelopes, measurement quality and
//! independence from the diagnostic evidence path. It does not authorize work,
//! authenticate issuers, prove causal effects, or establish durable safe life.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const REPAIR_RESTORATION_SCHEMA: &str = "symtropy-marine-repair-restoration-v0.1";
pub const REPAIR_RESTORATION_CASES_SCHEMA: &str = "symtropy-marine-repair-restoration-cases-v0.1";
const TIME_TOLERANCE_SECONDS: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementQuality {
    Valid,
    Missing,
    Stale,
    Rejected,
    Conflicted,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationState {
    Verified,
    Unknown,
    Expired,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatingCondition {
    pub condition_id: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityMeasurement {
    pub measurement_id: String,
    pub source_id: String,
    pub measurement_evidence_sha256: String,
    pub metric_id: String,
    pub measurement_principle_id: String,
    pub dependency_domain_id: String,
    pub observed_at_seconds: f64,
    pub available_at_seconds: f64,
    pub value: f64,
    pub uncertainty_abs: f64,
    pub unit: String,
    pub quality: MeasurementQuality,
    pub calibration: CalibrationState,
    pub calibration_record_sha256: Option<String>,
    pub calibration_valid_until_seconds: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TestEpisode {
    pub episode_id: String,
    pub asset_id: String,
    pub configuration_sha256: String,
    pub started_at_seconds: f64,
    pub ended_at_seconds: f64,
    pub conditions: Vec<OperatingCondition>,
    pub measurements: Vec<CapabilityMeasurement>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionTolerance {
    pub condition_id: String,
    pub unit: String,
    pub maximum_absolute_difference: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEnvelope {
    pub capability_id: String,
    pub metric_id: String,
    pub unit: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub maximum_measurement_uncertainty: f64,
    pub targeted_by_intervention: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RepairVerificationBinding {
    pub asset_id: String,
    pub component_id: String,
    pub diagnostic_revision_receipt_sha256: String,
    pub acceptance_envelope_owner_ref: String,
    pub acceptance_envelope_sha256: String,
    pub configuration_before_sha256: String,
    pub configuration_after_sha256: String,
    pub diagnostic_measurement_principle_ids: Vec<String>,
    pub diagnostic_dependency_domain_ids: Vec<String>,
    pub condition_tolerances: Vec<ConditionTolerance>,
    pub capability_envelopes: Vec<CapabilityEnvelope>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterventionKind {
    PhysicalComponentRepair,
    SensorReplacementOrCalibration,
    ControllerOrConfigurationChange,
    InspectionOnly,
    ShamOrNoOp,
    Other,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RepairRecord {
    pub repair_id: String,
    pub component_id: String,
    pub intervention_kind: InterventionKind,
    pub action_description: String,
    /// A pointer to an external authorization record; this crate does not verify authority.
    pub authorization_record_ref: String,
    pub authorization_record_sha256: String,
    pub repair_record_sha256: String,
    pub started_at_seconds: f64,
    pub completed_at_seconds: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConcurrentChange {
    pub change_id: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RepairVerificationCase {
    pub schema_version: String,
    pub binding: RepairVerificationBinding,
    pub repair: RepairRecord,
    pub pre_repair_episode: TestEpisode,
    pub post_repair_episode: TestEpisode,
    pub concurrent_changes: Vec<ConcurrentChange>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeState {
    Within,
    Outside,
    BoundaryUncertain,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityResultStatus {
    Restored,
    NotRestored,
    Preserved,
    Regression,
    CollateralImprovement,
    ExistingDeficit,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CapabilityVerificationResult {
    pub capability_id: String,
    pub targeted_by_intervention: bool,
    pub pre_repair_state: EnvelopeState,
    pub post_repair_state: EnvelopeState,
    pub status: CapabilityResultStatus,
    pub pre_measurement_id: String,
    pub post_measurement_id: String,
    pub pre_interval: [f64; 2],
    pub post_interval: [f64; 2],
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestorationDisposition {
    RestoredWithinDeclaredEnvelope,
    PartialRestoration,
    NotRestored,
    NonTargetCapabilityRegression,
    NotEvaluable,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttributionStatus {
    BeforeAfterAssociationOnly,
    ConfoundedConcurrentChanges,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotEvaluableReason {
    NoTargetCapability,
    NoPreRepairDeficitObserved,
    OperatingConditionsNotMatched,
    MissingOrAmbiguousMeasurement,
    MeasurementQualityNotCurrent,
    CalibrationNotVerified,
    MeasurementUncertaintyTooLarge,
    NoIndependentVerificationPath,
    UncertaintyOverlapsAcceptanceBoundary,
    InterventionNotPhysicalComponentRepair,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RepairVerificationReport {
    pub schema_version: String,
    pub repair_id: String,
    pub asset_id: String,
    pub disposition: RestorationDisposition,
    pub attribution_status: AttributionStatus,
    pub not_evaluable_reason: Option<NotEvaluableReason>,
    pub matched_condition_ids: Vec<String>,
    pub capability_results: Vec<CapabilityVerificationResult>,
    pub pre_repair_episode_id: String,
    pub post_repair_episode_id: String,
    pub repair_record_sha256: String,
    pub diagnostic_revision_receipt_sha256: String,
    pub acceptance_envelope_sha256: String,
    pub case_sha256: String,
    pub physical_restoration_authorized: bool,
    pub maintenance_authorized: bool,
    pub operating_restriction_clearance_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairVerificationError {
    InvalidSchemaVersion,
    EmptyIdentity,
    InvalidDigest,
    InvalidTimestamp,
    InvalidBinding,
    DuplicateCondition(String),
    DuplicateCapability(String),
    DuplicateMeasurement(String),
    DuplicateMeasurementId(String),
    InvalidEnvelope(String),
    InvalidRepairChronology,
    EpisodeIdentityMismatch,
    EpisodeConfigurationMismatch,
    InvalidEpisodeWindow(String),
    InvalidMeasurement(String),
    InvalidConditionTolerance(String),
    Serialization,
}

impl fmt::Display for RepairVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchemaVersion => {
                write!(formatter, "unexpected repair-restoration schema version")
            },
            Self::EmptyIdentity => write!(formatter, "repair-restoration identity is empty"),
            Self::InvalidDigest => write!(
                formatter,
                "repair-restoration digest is not lowercase SHA-256"
            ),
            Self::InvalidTimestamp => write!(formatter, "repair-restoration timestamp is invalid"),
            Self::InvalidBinding => write!(
                formatter,
                "repair-restoration binding is incomplete or inconsistent"
            ),
            Self::DuplicateCondition(id) => {
                write!(formatter, "duplicate operating condition: {id}")
            }
            Self::DuplicateCapability(id) => write!(formatter, "duplicate capability: {id}"),
            Self::DuplicateMeasurement(id) => {
                write!(formatter, "duplicate measurement for metric: {id}")
            }
            Self::DuplicateMeasurementId(id) => {
                write!(formatter, "duplicate measurement identity: {id}")
            }
            Self::InvalidEnvelope(id) => {
                write!(formatter, "invalid capability acceptance envelope: {id}")
            }
            Self::InvalidRepairChronology => write!(
                formatter,
                "repair chronology does not separate pre- and post-repair episodes"
            ),
            Self::EpisodeIdentityMismatch => write!(
                formatter,
                "pre/post episode asset identity does not match repair binding"
            ),
            Self::EpisodeConfigurationMismatch => write!(
                formatter,
                "pre/post episode configuration does not match the bound repair transition"
            ),
            Self::InvalidEpisodeWindow(id) => {
                write!(formatter, "invalid test episode window: {id}")
            }
            Self::InvalidMeasurement(id) => {
                write!(formatter, "invalid measurement values or identity: {id}")
            }
            Self::InvalidConditionTolerance(id) => {
                write!(formatter, "invalid condition tolerance: {id}")
            }
            Self::Serialization => {
                write!(formatter, "failed to serialize repair-restoration evidence")
            }
        }
    }
}

impl std::error::Error for RepairVerificationError {}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, RepairVerificationError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RepairVerificationError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn capability_envelopes_sha256(
    envelopes: &[CapabilityEnvelope],
) -> Result<String, RepairVerificationError> {
    let mut canonical = envelopes.to_vec();
    canonical.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    sha256_serializable(b"symtropy-marine-capability-envelope-set-v1\0", &canonical)
}

fn repair_case_sha256(case: &RepairVerificationCase) -> Result<String, RepairVerificationError> {
    let mut canonical = case.clone();
    canonical
        .binding
        .diagnostic_measurement_principle_ids
        .sort();
    canonical.binding.diagnostic_dependency_domain_ids.sort();
    canonical
        .binding
        .condition_tolerances
        .sort_by(|left, right| left.condition_id.cmp(&right.condition_id));
    canonical
        .binding
        .capability_envelopes
        .sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    canonical
        .pre_repair_episode
        .conditions
        .sort_by(|left, right| left.condition_id.cmp(&right.condition_id));
    canonical
        .post_repair_episode
        .conditions
        .sort_by(|left, right| left.condition_id.cmp(&right.condition_id));
    canonical
        .pre_repair_episode
        .measurements
        .sort_by(|left, right| left.metric_id.cmp(&right.metric_id));
    canonical
        .post_repair_episode
        .measurements
        .sort_by(|left, right| left.metric_id.cmp(&right.metric_id));
    canonical
        .concurrent_changes
        .sort_by(|left, right| left.change_id.cmp(&right.change_id));
    sha256_serializable(b"symtropy-marine-repair-verification-case-v1\0", &canonical)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl RepairVerificationBinding {
    fn validate(&self) -> Result<(), RepairVerificationError> {
        if self.asset_id.trim().is_empty()
            || self.component_id.trim().is_empty()
            || self.diagnostic_measurement_principle_ids.is_empty()
            || self.diagnostic_dependency_domain_ids.is_empty()
            || self.condition_tolerances.is_empty()
            || self.capability_envelopes.is_empty()
        {
            return Err(RepairVerificationError::InvalidBinding);
        }
        if !is_sha256_hex(&self.diagnostic_revision_receipt_sha256)
            || !is_sha256_hex(&self.acceptance_envelope_sha256)
            || !is_sha256_hex(&self.configuration_before_sha256)
            || !is_sha256_hex(&self.configuration_after_sha256)
        {
            return Err(RepairVerificationError::InvalidDigest);
        }
        if self.acceptance_envelope_owner_ref.trim().is_empty() {
            return Err(RepairVerificationError::InvalidBinding);
        }
        if capability_envelopes_sha256(&self.capability_envelopes)?
            != self.acceptance_envelope_sha256
        {
            return Err(RepairVerificationError::InvalidBinding);
        }
        let mut conditions = BTreeSet::new();
        for item in &self.condition_tolerances {
            if item.condition_id.trim().is_empty()
                || item.unit.trim().is_empty()
                || !item.maximum_absolute_difference.is_finite()
                || item.maximum_absolute_difference < 0.0
            {
                return Err(RepairVerificationError::InvalidConditionTolerance(
                    item.condition_id.clone(),
                ));
            }
            if !conditions.insert(item.condition_id.clone()) {
                return Err(RepairVerificationError::DuplicateCondition(
                    item.condition_id.clone(),
                ));
            }
        }
        let mut capabilities = BTreeSet::new();
        let mut metrics = BTreeSet::new();
        let mut targeted = false;
        for envelope in &self.capability_envelopes {
            if envelope.capability_id.trim().is_empty()
                || envelope.metric_id.trim().is_empty()
                || envelope.unit.trim().is_empty()
                || !envelope.lower_bound.is_finite()
                || !envelope.upper_bound.is_finite()
                || envelope.lower_bound >= envelope.upper_bound
                || !envelope.maximum_measurement_uncertainty.is_finite()
                || envelope.maximum_measurement_uncertainty < 0.0
            {
                return Err(RepairVerificationError::InvalidEnvelope(
                    envelope.capability_id.clone(),
                ));
            }
            if !capabilities.insert(envelope.capability_id.clone()) {
                return Err(RepairVerificationError::DuplicateCapability(
                    envelope.capability_id.clone(),
                ));
            }
            if !metrics.insert(envelope.metric_id.clone()) {
                return Err(RepairVerificationError::DuplicateMeasurement(
                    envelope.metric_id.clone(),
                ));
            }
            targeted |= envelope.targeted_by_intervention;
        }
        if !targeted {
            return Err(RepairVerificationError::InvalidBinding);
        }
        Ok(())
    }
}

fn validate_episode(episode: &TestEpisode) -> Result<(), RepairVerificationError> {
    if episode.episode_id.trim().is_empty() || episode.asset_id.trim().is_empty() {
        return Err(RepairVerificationError::EmptyIdentity);
    }
    if !is_sha256_hex(&episode.configuration_sha256) {
        return Err(RepairVerificationError::InvalidDigest);
    }
    if !episode.started_at_seconds.is_finite()
        || !episode.ended_at_seconds.is_finite()
        || episode.started_at_seconds < 0.0
        || episode.ended_at_seconds <= episode.started_at_seconds
    {
        return Err(RepairVerificationError::InvalidEpisodeWindow(
            episode.episode_id.clone(),
        ));
    }
    let mut condition_ids = BTreeSet::new();
    for condition in &episode.conditions {
        if condition.condition_id.trim().is_empty()
            || condition.unit.trim().is_empty()
            || !condition.value.is_finite()
            || !condition_ids.insert(condition.condition_id.clone())
        {
            return Err(RepairVerificationError::InvalidEpisodeWindow(
                episode.episode_id.clone(),
            ));
        }
    }
    let mut metric_ids = BTreeSet::new();
    let mut measurement_ids = BTreeSet::new();
    for measurement in &episode.measurements {
        if !measurement.observed_at_seconds.is_finite()
            || !measurement.available_at_seconds.is_finite()
            || measurement.observed_at_seconds < episode.started_at_seconds
            || measurement.observed_at_seconds > episode.ended_at_seconds
            || measurement.available_at_seconds < measurement.observed_at_seconds
            || measurement.available_at_seconds > episode.ended_at_seconds
        {
            return Err(RepairVerificationError::InvalidEpisodeWindow(
                episode.episode_id.clone(),
            ));
        }
        let calibration_digest_invalid = measurement
            .calibration_record_sha256
            .as_deref()
            .is_some_and(|digest| !is_sha256_hex(digest));
        let verified_without_digest = measurement.calibration == CalibrationState::Verified
            && (measurement.calibration_record_sha256.is_none()
                || measurement
                    .calibration_valid_until_seconds
                    .is_none_or(|valid_until| !valid_until.is_finite() || valid_until < measurement.available_at_seconds));
        let invalid_calibration_expiry = measurement
            .calibration_valid_until_seconds
            .is_some_and(|valid_until| !valid_until.is_finite());
        if measurement.measurement_id.trim().is_empty()
            || measurement.source_id.trim().is_empty()
            || !is_sha256_hex(&measurement.measurement_evidence_sha256)
            || measurement.metric_id.trim().is_empty()
            || measurement.measurement_principle_id.trim().is_empty()
            || measurement.dependency_domain_id.trim().is_empty()
            || measurement.unit.trim().is_empty()
            || !measurement.value.is_finite()
            || !measurement.uncertainty_abs.is_finite()
            || measurement.uncertainty_abs < 0.0
            || calibration_digest_invalid
            || verified_without_digest
            || invalid_calibration_expiry
        {
            return Err(RepairVerificationError::InvalidMeasurement(
                measurement.measurement_id.clone(),
            ));
        }
        if !metric_ids.insert(measurement.metric_id.clone()) {
            return Err(RepairVerificationError::DuplicateMeasurement(
                measurement.metric_id.clone(),
            ));
        }
        if !measurement_ids.insert(measurement.measurement_id.clone()) {
            return Err(RepairVerificationError::DuplicateMeasurementId(
                measurement.measurement_id.clone(),
            ));
        }
    }
    Ok(())
}

fn not_evaluable_report(
    case: &RepairVerificationCase,
    reason: NotEvaluableReason,
    matched_condition_ids: Vec<String>,
    case_sha256: String,
) -> RepairVerificationReport {
    RepairVerificationReport {
        schema_version: REPAIR_RESTORATION_SCHEMA.to_owned(),
        repair_id: case.repair.repair_id.clone(),
        asset_id: case.binding.asset_id.clone(),
        disposition: RestorationDisposition::NotEvaluable,
        attribution_status: if case.concurrent_changes.is_empty() {
            AttributionStatus::BeforeAfterAssociationOnly
        } else {
            AttributionStatus::ConfoundedConcurrentChanges
        },
        not_evaluable_reason: Some(reason),
        matched_condition_ids,
        capability_results: Vec::new(),
        pre_repair_episode_id: case.pre_repair_episode.episode_id.clone(),
        post_repair_episode_id: case.post_repair_episode.episode_id.clone(),
        repair_record_sha256: case.repair.repair_record_sha256.clone(),
        diagnostic_revision_receipt_sha256: case.binding.diagnostic_revision_receipt_sha256.clone(),
        acceptance_envelope_sha256: case.binding.acceptance_envelope_sha256.clone(),
        case_sha256,
        physical_restoration_authorized: false,
        maintenance_authorized: false,
        operating_restriction_clearance_authorized: false,
    }
}

fn validate_repair(case: &RepairVerificationCase) -> Result<(), RepairVerificationError> {
    if case.schema_version != REPAIR_RESTORATION_SCHEMA {
        return Err(RepairVerificationError::InvalidSchemaVersion);
    }
    case.binding.validate()?;
    if case.repair.repair_id.trim().is_empty()
        || case.repair.component_id != case.binding.component_id
        || case.repair.action_description.trim().is_empty()
        || case.repair.authorization_record_ref.trim().is_empty()
    {
        return Err(RepairVerificationError::EmptyIdentity);
    }
    if !is_sha256_hex(&case.repair.authorization_record_sha256)
        || !is_sha256_hex(&case.repair.repair_record_sha256)
    {
        return Err(RepairVerificationError::InvalidDigest);
    }
    if !case.repair.started_at_seconds.is_finite()
        || !case.repair.completed_at_seconds.is_finite()
        || case.repair.started_at_seconds < 0.0
        || case.repair.completed_at_seconds <= case.repair.started_at_seconds
    {
        return Err(RepairVerificationError::InvalidTimestamp);
    }
    validate_episode(&case.pre_repair_episode)?;
    validate_episode(&case.post_repair_episode)?;
    if case.pre_repair_episode.asset_id != case.binding.asset_id
        || case.post_repair_episode.asset_id != case.binding.asset_id
    {
        return Err(RepairVerificationError::EpisodeIdentityMismatch);
    }
    if case.pre_repair_episode.configuration_sha256 != case.binding.configuration_before_sha256
        || case.post_repair_episode.configuration_sha256 != case.binding.configuration_after_sha256
    {
        return Err(RepairVerificationError::EpisodeConfigurationMismatch);
    }
    if case.pre_repair_episode.ended_at_seconds
        > case.repair.started_at_seconds + TIME_TOLERANCE_SECONDS
        || case.post_repair_episode.started_at_seconds
            < case.repair.completed_at_seconds - TIME_TOLERANCE_SECONDS
    {
        return Err(RepairVerificationError::InvalidRepairChronology);
    }
    Ok(())
}

fn condition_map(episode: &TestEpisode) -> BTreeMap<&str, (&str, f64)> {
    episode
        .conditions
        .iter()
        .map(|condition| {
            (
                condition.condition_id.as_str(),
                (condition.unit.as_str(), condition.value),
            )
        })
        .collect()
}

fn conditions_match(
    binding: &RepairVerificationBinding,
    pre: &TestEpisode,
    post: &TestEpisode,
) -> bool {
    let pre_conditions = condition_map(pre);
    let post_conditions = condition_map(post);
    binding.condition_tolerances.iter().all(|tolerance| {
        let Some((pre_unit, pre_value)) = pre_conditions.get(tolerance.condition_id.as_str())
        else {
            return false;
        };
        let Some((post_unit, post_value)) = post_conditions.get(tolerance.condition_id.as_str())
        else {
            return false;
        };
        *pre_unit == tolerance.unit.as_str()
            && *post_unit == tolerance.unit.as_str()
            && (*pre_value - *post_value).abs()
                <= tolerance.maximum_absolute_difference + TIME_TOLERANCE_SECONDS
    })
}

fn envelope_state(
    measurement: &CapabilityMeasurement,
    envelope: &CapabilityEnvelope,
) -> EnvelopeState {
    let lower = measurement.value - measurement.uncertainty_abs;
    let upper = measurement.value + measurement.uncertainty_abs;
    if lower >= envelope.lower_bound && upper <= envelope.upper_bound {
        EnvelopeState::Within
    } else if upper < envelope.lower_bound || lower > envelope.upper_bound {
        EnvelopeState::Outside
    } else {
        EnvelopeState::BoundaryUncertain
    }
}

fn valid_measurement(
    measurement: &CapabilityMeasurement,
    envelope: &CapabilityEnvelope,
) -> Result<(), NotEvaluableReason> {
    if measurement.quality != MeasurementQuality::Valid {
        return Err(NotEvaluableReason::MeasurementQualityNotCurrent);
    }
    if measurement.calibration != CalibrationState::Verified {
        return Err(NotEvaluableReason::CalibrationNotVerified);
    }
    if measurement.unit != envelope.unit {
        return Err(NotEvaluableReason::MissingOrAmbiguousMeasurement);
    }
    if measurement.uncertainty_abs > envelope.maximum_measurement_uncertainty {
        return Err(NotEvaluableReason::MeasurementUncertaintyTooLarge);
    }
    Ok(())
}

fn independent_verification_measurement(
    measurement: &CapabilityMeasurement,
    binding: &RepairVerificationBinding,
) -> bool {
    !binding
        .diagnostic_measurement_principle_ids
        .contains(&measurement.measurement_principle_id)
        && !binding
            .diagnostic_dependency_domain_ids
            .contains(&measurement.dependency_domain_id)
}

pub fn evaluate_repair_restoration(
    case: &RepairVerificationCase,
) -> Result<RepairVerificationReport, RepairVerificationError> {
    validate_repair(case)?;
    let case_sha256 = repair_case_sha256(case)?;
    let binding = &case.binding;
    let pre = &case.pre_repair_episode;
    let post = &case.post_repair_episode;

    if case.repair.intervention_kind != InterventionKind::PhysicalComponentRepair {
        return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::InterventionNotPhysicalComponentRepair,
            Vec::new(),
            case_sha256.clone(),
        ));
    }

    if !conditions_match(binding, pre, post) {
        return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::OperatingConditionsNotMatched,
            Vec::new(),
            case_sha256.clone(),
        ));
    }

    let mut results = Vec::with_capacity(binding.capability_envelopes.len());
    let mut matched_condition_ids = binding
        .condition_tolerances
        .iter()
        .map(|item| item.condition_id.clone())
        .collect::<Vec<_>>();
    matched_condition_ids.sort();

    for envelope in &binding.capability_envelopes {
        let Some(pre_measurement) = pre
            .measurements
            .iter()
            .find(|measurement| measurement.metric_id == envelope.metric_id)
        else {
            return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::MissingOrAmbiguousMeasurement,
            matched_condition_ids,
            case_sha256.clone(),
        ));
        };
        let Some(post_measurement) = post
            .measurements
            .iter()
            .find(|measurement| measurement.metric_id == envelope.metric_id)
        else {
            return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::MissingOrAmbiguousMeasurement,
            matched_condition_ids,
            case_sha256.clone(),
        ));
        };

        if let Err(reason) = valid_measurement(pre_measurement, envelope) {
            return Ok(not_evaluable_report(
                case,
                reason,
                matched_condition_ids,
                case_sha256.clone(),
            ));
        }
        if let Err(reason) = valid_measurement(post_measurement, envelope) {
            return Ok(not_evaluable_report(
                case,
                reason,
                matched_condition_ids,
                case_sha256.clone(),
            ));
        }
        let same_reference_measurement_path = pre_measurement.measurement_principle_id
            == post_measurement.measurement_principle_id
            && pre_measurement.dependency_domain_id == post_measurement.dependency_domain_id;
        if !independent_verification_measurement(pre_measurement, binding)
            || !independent_verification_measurement(post_measurement, binding)
            || !same_reference_measurement_path
        {
            return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::NoIndependentVerificationPath,
            matched_condition_ids,
            case_sha256.clone(),
        ));
        }

        let pre_state = envelope_state(pre_measurement, envelope);
        let post_state = envelope_state(post_measurement, envelope);
        if pre_state == EnvelopeState::BoundaryUncertain
            || post_state == EnvelopeState::BoundaryUncertain
        {
            return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::UncertaintyOverlapsAcceptanceBoundary,
            matched_condition_ids,
            case_sha256.clone(),
        ));
        }
        if envelope.targeted_by_intervention && pre_state != EnvelopeState::Outside {
            return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::NoPreRepairDeficitObserved,
            matched_condition_ids,
            case_sha256.clone(),
        ));
        }

        let status = match (envelope.targeted_by_intervention, pre_state, post_state) {
            (true, EnvelopeState::Outside, EnvelopeState::Within) => {
                CapabilityResultStatus::Restored
            }
            (true, EnvelopeState::Outside, EnvelopeState::Outside) => {
                CapabilityResultStatus::NotRestored
            }
            (false, EnvelopeState::Within, EnvelopeState::Outside) => {
                CapabilityResultStatus::Regression
            }
            (false, EnvelopeState::Within, EnvelopeState::Within) => {
                CapabilityResultStatus::Preserved
            }
            (false, EnvelopeState::Outside, EnvelopeState::Within) => {
                CapabilityResultStatus::CollateralImprovement
            }
            (false, EnvelopeState::Outside, EnvelopeState::Outside) => {
                CapabilityResultStatus::ExistingDeficit
            }
            _ => {
                return Ok(not_evaluable_report(
            case,
            NotEvaluableReason::NoPreRepairDeficitObserved,
            matched_condition_ids,
            case_sha256.clone(),
        ));
            }
        };

        results.push(CapabilityVerificationResult {
            capability_id: envelope.capability_id.clone(),
            targeted_by_intervention: envelope.targeted_by_intervention,
            pre_repair_state: pre_state,
            post_repair_state: post_state,
            status,
            pre_measurement_id: pre_measurement.measurement_id.clone(),
            post_measurement_id: post_measurement.measurement_id.clone(),
            pre_interval: [
                pre_measurement.value - pre_measurement.uncertainty_abs,
                pre_measurement.value + pre_measurement.uncertainty_abs,
            ],
            post_interval: [
                post_measurement.value - post_measurement.uncertainty_abs,
                post_measurement.value + post_measurement.uncertainty_abs,
            ],
        });
    }

    let any_regression = results
        .iter()
        .any(|item| item.status == CapabilityResultStatus::Regression);
    let targeted = results
        .iter()
        .filter(|item| item.targeted_by_intervention)
        .collect::<Vec<_>>();
    let restored_count = targeted
        .iter()
        .filter(|item| item.status == CapabilityResultStatus::Restored)
        .count();
    let all_targeted_restored = restored_count == targeted.len();
    let all_required_within = results.iter().all(|item| {
        matches!(
            item.status,
            CapabilityResultStatus::Restored
                | CapabilityResultStatus::Preserved
                | CapabilityResultStatus::CollateralImprovement
        )
    });

    let disposition = if any_regression {
        RestorationDisposition::NonTargetCapabilityRegression
    } else if all_targeted_restored && all_required_within {
        RestorationDisposition::RestoredWithinDeclaredEnvelope
    } else if restored_count > 0 {
        RestorationDisposition::PartialRestoration
    } else {
        RestorationDisposition::NotRestored
    };

    Ok(RepairVerificationReport {
        schema_version: REPAIR_RESTORATION_SCHEMA.to_owned(),
        repair_id: case.repair.repair_id.clone(),
        asset_id: binding.asset_id.clone(),
        disposition,
        attribution_status: if case.concurrent_changes.is_empty() {
            AttributionStatus::BeforeAfterAssociationOnly
        } else {
            AttributionStatus::ConfoundedConcurrentChanges
        },
        not_evaluable_reason: None,
        matched_condition_ids,
        capability_results: results,
        pre_repair_episode_id: pre.episode_id.clone(),
        post_repair_episode_id: post.episode_id.clone(),
        repair_record_sha256: case.repair.repair_record_sha256.clone(),
        diagnostic_revision_receipt_sha256: binding.diagnostic_revision_receipt_sha256.clone(),
        acceptance_envelope_sha256: binding.acceptance_envelope_sha256.clone(),
        case_sha256,
        physical_restoration_authorized: false,
        maintenance_authorized: false,
        operating_restriction_clearance_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn measurement(
        measurement_id: &str,
        metric_id: &str,
        value: f64,
        measurement_principle_id: &str,
        dependency_domain_id: &str,
    ) -> CapabilityMeasurement {
        CapabilityMeasurement {
            measurement_id: measurement_id.into(),
            source_id: measurement_id.into(),
            measurement_evidence_sha256: digest('e'),
            metric_id: metric_id.into(),
            measurement_principle_id: measurement_principle_id.into(),
            dependency_domain_id: dependency_domain_id.into(),
            observed_at_seconds: 0.0,
            available_at_seconds: 0.0,
            value,
            uncertainty_abs: 0.5,
            unit: "kPa".into(),
            quality: MeasurementQuality::Valid,
            calibration: CalibrationState::Verified,
            calibration_record_sha256: Some(digest('d')),
            calibration_valid_until_seconds: Some(1000.0),
        }
    }

    fn episode(
        id: &str,
        configuration: String,
        start: f64,
        end: f64,
        load: f64,
        torque: f64,
        cooling: f64,
        principles: [&str; 2],
        domains: [&str; 2],
    ) -> TestEpisode {
        let mut torque_measurement = measurement(
            "m-torque",
            "propulsion_delivery_pressure",
            torque,
            principles[0],
            domains[0],
        );
        torque_measurement.source_id = format!("{id}-torque-meter");
        torque_measurement.observed_at_seconds = end - 1.0;
        torque_measurement.available_at_seconds = end;
        let mut cooling_measurement = measurement(
            "m-cooling",
            "cooling_pressure",
            cooling,
            principles[1],
            domains[1],
        );
        cooling_measurement.source_id = format!("{id}-cooling-meter");
        cooling_measurement.observed_at_seconds = end - 1.0;
        cooling_measurement.available_at_seconds = end;

        TestEpisode {
            episode_id: id.into(),
            asset_id: "engine-001".into(),
            configuration_sha256: configuration,
            started_at_seconds: start,
            ended_at_seconds: end,
            conditions: vec![
                OperatingCondition {
                    condition_id: "load_percent".into(),
                    value: load,
                    unit: "percent".into(),
                },
                OperatingCondition {
                    condition_id: "engine_speed".into(),
                    value: 1500.0,
                    unit: "rpm".into(),
                },
            ],
            measurements: vec![torque_measurement, cooling_measurement],
        }
    }

    fn valid_case() -> RepairVerificationCase {
        let capability_envelopes = vec![
            CapabilityEnvelope {
                capability_id: "propulsion-delivery".into(),
                metric_id: "propulsion_delivery_pressure".into(),
                unit: "kPa".into(),
                lower_bound: 90.0,
                upper_bound: 120.0,
                maximum_measurement_uncertainty: 2.0,
                targeted_by_intervention: true,
            },
            CapabilityEnvelope {
                capability_id: "cooling-loop".into(),
                metric_id: "cooling_pressure".into(),
                unit: "kPa".into(),
                lower_bound: 20.0,
                upper_bound: 30.0,
                maximum_measurement_uncertainty: 2.0,
                targeted_by_intervention: false,
            },
        ];
        let acceptance_envelope_sha256 =
            capability_envelopes_sha256(&capability_envelopes).unwrap();
        let binding = RepairVerificationBinding {
            asset_id: "engine-001".into(),
            component_id: "fuel-injection-pump".into(),
            diagnostic_revision_receipt_sha256: digest('f'),
            acceptance_envelope_owner_ref: "OEM-envelope-rev-7".into(),
            acceptance_envelope_sha256,
            configuration_before_sha256: digest('a'),
            configuration_after_sha256: digest('b'),
            diagnostic_measurement_principle_ids: vec!["engine-speed-sensor".into()],
            diagnostic_dependency_domain_ids: vec!["control-gateway".into()],
            condition_tolerances: vec![
                ConditionTolerance {
                    condition_id: "load_percent".into(),
                    unit: "percent".into(),
                    maximum_absolute_difference: 1.0,
                },
                ConditionTolerance {
                    condition_id: "engine_speed".into(),
                    unit: "rpm".into(),
                    maximum_absolute_difference: 10.0,
                },
            ],
            capability_envelopes,
        };
        RepairVerificationCase {
            schema_version: REPAIR_RESTORATION_SCHEMA.into(),
            binding,
            repair: RepairRecord {
                repair_id: "repair-001".into(),
                component_id: "fuel-injection-pump".into(),
                intervention_kind: InterventionKind::PhysicalComponentRepair,
                action_description: "Replace worn metering assembly".into(),
                authorization_record_ref: "work-order-ref-001".into(),
                authorization_record_sha256: digest('a'),
                repair_record_sha256: digest('c'),
                started_at_seconds: 10.0,
                completed_at_seconds: 11.0,
            },
            pre_repair_episode: episode(
                "pre",
                digest('a'),
                0.0,
                9.0,
                75.0,
                80.0,
                25.0,
                ["torque-flange", "pressure-tap"],
                ["independent-daq", "independent-pressure-logger"],
            ),
            post_repair_episode: episode(
                "post",
                digest('b'),
                12.0,
                20.0,
                75.5,
                110.0,
                25.0,
                ["torque-flange", "pressure-tap"],
                ["independent-daq", "independent-pressure-logger"],
            ),
            concurrent_changes: Vec::new(),
        }
    }

    #[test]
    fn matched_independent_post_repair_measurements_verify_target_capability() {
        let report = evaluate_repair_restoration(&valid_case()).unwrap();
        assert_eq!(
            report.disposition,
            RestorationDisposition::RestoredWithinDeclaredEnvelope
        );
        assert_eq!(
            report.attribution_status,
            AttributionStatus::BeforeAfterAssociationOnly
        );
        assert_eq!(report.capability_results.len(), 2);
        assert!(!report.physical_restoration_authorized);
        assert!(!report.maintenance_authorized);
        assert!(!report.operating_restriction_clearance_authorized);
    }

    #[test]
    fn canonical_case_digest_is_invariant_to_set_like_measurement_order() {
        let case = valid_case();
        let first_digest = repair_case_sha256(&case).unwrap();
        let mut reordered = case;
        reordered.pre_repair_episode.measurements.reverse();
        reordered.binding.capability_envelopes.reverse();
        assert_eq!(first_digest, repair_case_sha256(&reordered).unwrap());
    }

    #[test]
    fn sensor_only_intervention_cannot_be_reported_as_physical_restoration() {
        let mut case = valid_case();
        case.repair.intervention_kind = InterventionKind::SensorReplacementOrCalibration;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::InterventionNotPhysicalComponentRepair)
        );
    }

    #[test]
    fn repair_record_alone_does_not_establish_restoration() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].value = 80.0;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotRestored);
    }

    #[test]
    fn different_load_or_regime_is_not_a_valid_before_after_comparison() {
        let mut case = valid_case();
        case.post_repair_episode.conditions[0].value = 35.0;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::OperatingConditionsNotMatched)
        );
    }

    #[test]
    fn same_failed_measurement_dependency_cannot_verify_physical_restoration() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].dependency_domain_id = "control-gateway".into();
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::NoIndependentVerificationPath)
        );
    }

    #[test]
    fn non_target_capability_regression_is_non_compensatory() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[1].value = 40.0;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(
            report.disposition,
            RestorationDisposition::NonTargetCapabilityRegression
        );
        assert!(
            report
                .capability_results
                .iter()
                .any(|item| item.status == CapabilityResultStatus::Regression)
        );
    }

    #[test]
    fn healthy_pre_repair_target_cannot_be_claimed_as_restored_by_this_intervention() {
        let mut case = valid_case();
        case.pre_repair_episode.measurements[0].value = 110.0;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::NoPreRepairDeficitObserved)
        );
    }

    #[test]
    fn restoration_of_only_one_of_two_target_capabilities_is_partial() {
        let mut case = valid_case();
        case.binding.capability_envelopes[1].targeted_by_intervention = true;
        case.pre_repair_episode.measurements[1].value = 40.0;
        case.post_repair_episode.measurements[1].value = 10.0;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(
            report.disposition,
            RestorationDisposition::PartialRestoration
        );
    }

    #[test]
    fn uncertainty_overlapping_acceptance_boundary_is_not_called_a_pass() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].value = 91.0;
        case.post_repair_episode.measurements[0].uncertainty_abs = 2.0;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::UncertaintyOverlapsAcceptanceBoundary)
        );
    }

    #[test]
    fn concurrent_changes_prevent_isolated_causal_attribution() {
        let mut case = valid_case();
        case.concurrent_changes.push(ConcurrentChange {
            change_id: "controller-update".into(),
            description: "Controller map updated during the same maintenance window".into(),
        });
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(
            report.disposition,
            RestorationDisposition::RestoredWithinDeclaredEnvelope
        );
        assert_eq!(
            report.attribution_status,
            AttributionStatus::ConfoundedConcurrentChanges
        );
    }

    #[test]
    fn physical_repair_must_finish_before_post_repair_test_and_match_configuration() {
        let mut case = valid_case();
        case.repair.completed_at_seconds = 15.0;
        assert_eq!(
            evaluate_repair_restoration(&case),
            Err(RepairVerificationError::InvalidRepairChronology)
        );
    }

    #[test]
    fn missing_or_expired_post_repair_measurements_are_not_healthy() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].quality = MeasurementQuality::Missing;
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::MeasurementQualityNotCurrent)
        );
    }

    #[test]
    fn post_repair_evidence_available_after_its_test_window_is_rejected() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].available_at_seconds = 21.0;
        assert_eq!(
            evaluate_repair_restoration(&case),
            Err(RepairVerificationError::InvalidEpisodeWindow("post".into()))
        );
    }

    #[test]
    fn verified_measurement_without_calibration_record_digest_is_rejected() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].calibration_record_sha256 = None;
        assert_eq!(
            evaluate_repair_restoration(&case),
            Err(RepairVerificationError::InvalidEpisodeWindow("post".into()))
        );
    }

    #[test]
    fn pre_and_post_verification_use_the_same_independent_reference_path() {
        let mut case = valid_case();
        case.pre_repair_episode.measurements[0].measurement_principle_id =
            "diagnostic-engine-speed".into();
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::NoIndependentVerificationPath)
        );
    }

    #[test]
    fn pre_repair_measurement_sharing_diagnostic_path_is_not_an_independent_baseline() {
        let mut case = valid_case();
        case.pre_repair_episode.measurements[0].dependency_domain_id = "control-gateway".into();
        let report = evaluate_repair_restoration(&case).unwrap();
        assert_eq!(report.disposition, RestorationDisposition::NotEvaluable);
        assert_eq!(
            report.not_evaluable_reason,
            Some(NotEvaluableReason::NoIndependentVerificationPath)
        );
    }

    #[test]
    fn post_repair_evidence_available_after_its_test_window_is_rejected() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].available_at_seconds = 21.0;
        assert_eq!(
            evaluate_repair_restoration(&case),
            Err(RepairVerificationError::InvalidEpisodeWindow("post".into()))
        );
    }

    #[test]
    fn verified_measurement_without_calibration_record_digest_is_rejected() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].calibration_record_sha256 = None;
        assert_eq!(
            evaluate_repair_restoration(&case),
            Err(RepairVerificationError::InvalidMeasurement(
                "m-torque".into()
            ))
        );
    }

    #[test]
    fn calibration_expiring_before_post_measurement_availability_is_rejected() {
        let mut case = valid_case();
        case.post_repair_episode.measurements[0].calibration_valid_until_seconds = Some(19.0);
        assert_eq!(
            evaluate_repair_restoration(&case),
            Err(RepairVerificationError::InvalidMeasurement("m-torque".into()))
        );
    }

    #[test]
    fn acceptance_envelope_digest_must_match_owner_bound_envelopes() {
        let mut case = valid_case();
        case.binding.acceptance_envelope_sha256 = digest('9');
        assert_eq!(
            evaluate_repair_restoration(&case),
            Err(RepairVerificationError::InvalidBinding)
        );
    }

    #[test]
    fn protocol_and_corpus_keep_non_compensatory_contract_machine_readable() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../repair-restoration-v0.1.json")).unwrap();
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../repair-restoration-cases-v0.1.json")).unwrap();
        assert_eq!(protocol["schema_version"], REPAIR_RESTORATION_SCHEMA);
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert!(protocol["hard_gates"].as_array().unwrap().len() >= 12);
        assert_eq!(corpus["schema_version"], REPAIR_RESTORATION_CASES_SCHEMA);
        assert_eq!(corpus["cases"].as_array().unwrap().len(), 12);
        assert!(
            corpus["cases"]
                .as_array()
                .unwrap()
                .iter()
                .all(|case| case["expected_disposition"].is_string())
        );
    }
}
