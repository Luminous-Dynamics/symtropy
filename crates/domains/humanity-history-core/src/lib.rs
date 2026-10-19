// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Renderer-independent records and validation for evidence-aware historical reconstructions.
//!
//! This crate deliberately does not depend on Bevy, a database, a network client, or an AI
//! runtime. It validates the integrity of a small historical evidence package before a renderer
//! or research assistant consumes it. Validation is not a verdict that a historical claim is
//! true: source quality and scholarly interpretation remain reviewable research questions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Schema version for the first interoperable record format.
pub const SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dataset {
    pub schema_version: String,
    #[serde(default)]
    pub sources: Vec<SourceRecord>,
    #[serde(default)]
    pub claims: Vec<HistoricalClaim>,
    #[serde(default)]
    pub relations: Vec<KnowledgeRelation>,
    #[serde(default)]
    pub assets: Vec<AssetRecord>,
    /// Reported numeric observations, estimates, and reconstruction parameters with assumptions.
    #[serde(default)]
    pub estimates: Vec<QuantitativeEstimate>,
    /// Published interpretations or candidate designs, kept separate rather than averaged together.
    #[serde(default)]
    pub reconstruction_models: Vec<ReconstructionModel>,
    /// Explicit comparison sets; no single winner is inferred by the validator.
    #[serde(default)]
    pub model_comparisons: Vec<ModelComparison>,
    /// Explicit descriptive point-vs-interval sensitivity probes. These are not model probabilities.
    #[serde(default)]
    pub estimate_contrasts: Vec<EstimateContrast>,
    /// Paired point estimates used to quantify change across an explicitly named data/method variant.
    #[serde(default)]
    pub estimate_deltas: Vec<EstimateDelta>,
    /// Execution manifests for model runs; a claim of reproduction requires a traceable run record.
    #[serde(default)]
    pub analysis_runs: Vec<AnalysisRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRecord {
    /// Stable identifier in this dataset; preserve source-native IDs where possible.
    pub id: String,
    pub title: String,
    /// Stable URI, catalogue identifier, or archival reference. This crate does not fetch it.
    pub source_uri: String,
    pub rights: RightsDeclaration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightsDeclaration {
    pub status: RightsStatus,
    /// Required when status is `open_license`; preserve the exact license identifier.
    #[serde(default)]
    pub license_id: Option<String>,
    /// Human-readable attribution to display when required by a license or community.
    #[serde(default)]
    pub attribution: Option<String>,
    /// Record the basis for a permission or other non-obvious rights decision.
    #[serde(default)]
    pub basis_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightsStatus {
    PublicDomain,
    OpenLicense,
    PermissionGranted,
    Restricted,
    Unknown,
    CommunityProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalClaim {
    pub id: String,
    pub proposition: String,
    pub mode: ClaimMode,
    #[serde(default)]
    pub valid_time: Option<TimeInterval>,
    #[serde(default)]
    pub evidence: Vec<EvidenceLink>,
    /// Required for hypotheses and counterfactuals; states why the interpretation is proposed.
    #[serde(default)]
    pub interpretation_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimMode {
    Attested,
    EvidenceBasedReconstruction,
    Hypothesis,
    Counterfactual,
}

impl ClaimMode {
    fn requires_evidence(self) -> bool {
        matches!(self, Self::Attested | Self::EvidenceBasedReconstruction)
    }

    fn requires_interpretation_note(self) -> bool {
        matches!(self, Self::Hypothesis | Self::Counterfactual)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeInterval {
    /// Calendar identifier or documented calendar convention. No implicit conversion is performed.
    pub calendar: String,
    /// Ordinal year value within the named calendar; not necessarily Gregorian.
    pub start_year: i32,
    pub end_year: i32,
    pub precision: DatePrecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatePrecision {
    ExactYear,
    ApproximateYear,
    Decade,
    Century,
    BroadRange,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceLink {
    pub source_id: String,
    /// Page, paragraph, object number, figure, catalogue field, timestamp, or other source anchor.
    pub locator: String,
    pub relation: EvidenceRelation,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelation {
    Supports,
    Challenges,
    Qualifies,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRelation {
    pub id: String,
    pub from_claim_id: String,
    pub to_claim_id: String,
    pub kind: RelationKind,
    pub status: RelationStatus,
    #[serde(default)]
    pub evidence: Vec<EvidenceLink>,
    /// Hypothesized or counterfactual edges must explain their interpretive basis.
    #[serde(default)]
    pub interpretation_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Precedes,
    Influenced,
    IndependentlyDeveloped,
    TransmittedVia,
    DerivedFrom,
    Enabled,
    Supports,
    Challenges,
    Qualifies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationStatus {
    Documented,
    Hypothesized,
    Counterfactual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: String,
    /// Source or collection record establishing the asset's origin/provenance.
    pub source_id: String,
    pub rights: RightsDeclaration,
    /// Assets intended for distribution are subject to fail-closed rights validation.
    pub distributable: bool,
    /// SHA-256 of the exact asset bytes, represented by 64 hexadecimal characters.
    pub content_sha256: String,
}

/// A content-addressed file included in an analysis run's input or output manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunArtifactDigest {
    pub name: String,
    pub sha256: String,
}

/// Status is deliberately lifecycle-oriented: source-reported literature values do not count as
/// executed or reproduced local analyses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRunStatus {
    Planned,
    InputsVerified,
    Executed,
    Reproduced,
    Failed,
}

/// Versioned record of an experiment/reproduction attempt. The digest fields must be calculated
/// from actual bytes by the caller. This validator checks their syntax and required provenance
/// fields but cannot inspect external files or prove that a command was truly executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisRun {
    pub id: String,
    pub purpose: String,
    pub status: AnalysisRunStatus,
    pub model_spec_version: String,
    #[serde(default)]
    pub code_revision: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub environment_fingerprint: Option<String>,
    #[serde(default)]
    pub input_manifest_sha256: Option<String>,
    #[serde(default)]
    pub input_artifacts: Vec<RunArtifactDigest>,
    #[serde(default)]
    pub output_artifacts: Vec<RunArtifactDigest>,
    #[serde(default)]
    pub random_seed: Option<String>,
    #[serde(default)]
    pub target_estimate_ids: Vec<String>,
    #[serde(default)]
    pub comparison_source_id: Option<String>,
    /// Preregistered quantitative or qualitative rule used to compare this run with its target.
    #[serde(default)]
    pub comparison_rule: Option<String>,
    #[serde(default)]
    pub result_summary: Option<String>,
    pub limitations: Vec<String>,
}

/// A numerical result reported by a source or specified by a reconstruction.
///
/// Decimal values are strings to avoid silently rounding source-reported precision in JSON.
/// The validator checks numeric syntax and interval ordering; it does not recalculate the
/// source's statistical model or infer that a reported estimate is historically correct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantitativeEstimate {
    pub id: String,
    pub claim_id: String,
    pub quantity: String,
    pub value_decimal: String,
    pub unit: String,
    pub kind: EstimateKind,
    pub method: String,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub interval: Option<UncertaintyInterval>,
    #[serde(default)]
    pub evidence: Vec<EvidenceLink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateKind {
    ReportedPoint,
    ReportedIntervalEstimate,
    ObservedMeasurement,
    ReconstructionParameter,
    DerivedResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UncertaintyInterval {
    pub lower_decimal: String,
    pub upper_decimal: String,
    pub kind: IntervalKind,
    /// Coverage percentage such as "68" for a 68% credible interval; interpretation is method-specific.
    #[serde(default)]
    pub coverage_percent: Option<String>,
    /// Must record the condition when estimates depend on exclusions or alternative preprocessing.
    pub conditioning_note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntervalKind {
    Credible,
    Confidence,
    Measurement,
    PlausibleRange,
    SensitivityRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconstructionModel {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: ReconstructionModelStatus,
    pub method: String,
    pub target_claim_ids: Vec<String>,
    pub estimate_ids: Vec<String>,
    pub assumptions: Vec<String>,
    pub evidence: Vec<EvidenceLink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconstructionModelStatus {
    PublishedInterpretation,
    PublishedCandidateDesign,
    Hypothesis,
}

/// A declared comparison between a point-valued reconstruction parameter and a
/// source-reported uncertainty interval for the same quantity and unit.
///
/// This is intentionally a descriptive diagnostic. It does not calculate a Bayes factor,
/// posterior model probability, or truth score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstimateContrast {
    pub id: String,
    pub candidate_estimate_id: String,
    pub reference_estimate_id: String,
    pub purpose: String,
    /// Explain whether the analyses share data, methods, artifact assumptions, or dependencies.
    pub dependence_note: String,
    /// Explain why interval position must not be interpreted as a model probability.
    pub interpretation_limit: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntervalPosition {
    Below,
    Within,
    Above,
}

/// A point-vs-point sensitivity comparison, normally between a baseline and a declared
/// preprocessing/model variant. Differences are descriptive and do not imply independence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstimateDelta {
    pub id: String,
    pub baseline_estimate_id: String,
    pub perturbed_estimate_id: String,
    pub variant_description: String,
    pub dependence_note: String,
    pub interpretation_limit: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaDirection {
    Increase,
    Decrease,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstimateDeltaDiagnostic {
    pub delta_id: String,
    pub baseline_estimate_id: String,
    pub perturbed_estimate_id: String,
    pub quantity: String,
    pub unit: String,
    pub baseline_value_decimal: String,
    pub perturbed_value_decimal: String,
    /// Exact decimal-string difference: perturbed value minus baseline value.
    pub delta_value_decimal: String,
    pub absolute_delta_decimal: String,
    pub direction: DeltaDirection,
    pub variant_description: String,
    pub dependence_note: String,
    pub interpretation_limit: String,
}

pub fn analyze_estimate_delta(
    dataset: &Dataset,
    delta: &EstimateDelta,
) -> Result<EstimateDeltaDiagnostic, String> {
    let baseline = dataset
        .estimates
        .iter()
        .find(|estimate| estimate.id == delta.baseline_estimate_id)
        .ok_or_else(|| format!("missing baseline estimate '{}'", delta.baseline_estimate_id))?;
    let perturbed = dataset
        .estimates
        .iter()
        .find(|estimate| estimate.id == delta.perturbed_estimate_id)
        .ok_or_else(|| format!("missing perturbed estimate '{}'", delta.perturbed_estimate_id))?;

    if baseline.quantity != perturbed.quantity || baseline.unit != perturbed.unit {
        return Err("estimate delta requires identical quantity and unit".to_owned());
    }
    if baseline.interval.is_some() || perturbed.interval.is_some() {
        return Err("estimate delta requires two point-valued estimates; use an interval contrast otherwise".to_owned());
    }
    parse_decimal_parts(&baseline.value_decimal)
        .ok_or_else(|| "baseline value is not a finite plain decimal".to_owned())?;
    parse_decimal_parts(&perturbed.value_decimal)
        .ok_or_else(|| "perturbed value is not a finite plain decimal".to_owned())?;
    let delta_value_decimal = subtract_decimal_strings(&perturbed.value_decimal, &baseline.value_decimal)
        .ok_or_else(|| "could not calculate exact decimal difference".to_owned())?;
    let direction = match compare_decimal_strings(&perturbed.value_decimal, &baseline.value_decimal)
        .ok_or_else(|| "could not compare exact decimal values".to_owned())? {
        std::cmp::Ordering::Greater => DeltaDirection::Increase,
        std::cmp::Ordering::Less => DeltaDirection::Decrease,
        std::cmp::Ordering::Equal => DeltaDirection::Unchanged,
    };
    let absolute_delta_decimal = delta_value_decimal.strip_prefix('-')
        .unwrap_or(&delta_value_decimal)
        .to_owned();

    Ok(EstimateDeltaDiagnostic {
        delta_id: delta.id.clone(),
        baseline_estimate_id: baseline.id.clone(),
        perturbed_estimate_id: perturbed.id.clone(),
        quantity: baseline.quantity.clone(),
        unit: baseline.unit.clone(),
        baseline_value_decimal: baseline.value_decimal.clone(),
        perturbed_value_decimal: perturbed.value_decimal.clone(),
        delta_value_decimal,
        absolute_delta_decimal,
        direction,
        variant_description: delta.variant_description.clone(),
        dependence_note: delta.dependence_note.clone(),
        interpretation_limit: delta.interpretation_limit.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointIntervalDiagnostic {
    pub contrast_id: String,
    pub candidate_estimate_id: String,
    pub reference_estimate_id: String,
    pub quantity: String,
    pub unit: String,
    pub candidate_value_decimal: String,
    pub interval_lower_decimal: String,
    pub interval_upper_decimal: String,
    pub interval_kind: IntervalKind,
    pub coverage_percent: Option<String>,
    pub position: IntervalPosition,
    /// Exact decimal-string distance in the declared unit; zero means the point is within bounds.
    pub distance_to_interval_decimal: String,
    pub interpretation_limit: String,
}

pub fn analyze_estimate_contrast(
    dataset: &Dataset,
    contrast: &EstimateContrast,
) -> Result<PointIntervalDiagnostic, String> {
    let candidate = dataset
        .estimates
        .iter()
        .find(|estimate| estimate.id == contrast.candidate_estimate_id)
        .ok_or_else(|| format!("missing candidate estimate '{}'", contrast.candidate_estimate_id))?;
    let reference = dataset
        .estimates
        .iter()
        .find(|estimate| estimate.id == contrast.reference_estimate_id)
        .ok_or_else(|| format!("missing reference estimate '{}'", contrast.reference_estimate_id))?;

    if candidate.quantity != reference.quantity || candidate.unit != reference.unit {
        return Err("point-vs-interval contrast requires identical quantity and unit".to_owned());
    }
    if candidate.interval.is_some() {
        return Err("candidate estimate must be point-valued; choose a point parameter without an interval".to_owned());
    }
    let interval = reference
        .interval
        .as_ref()
        .ok_or_else(|| "reference estimate has no reported uncertainty interval".to_owned())?;
    parse_decimal_parts(&candidate.value_decimal)
        .ok_or_else(|| "candidate value is not a finite plain decimal".to_owned())?;
    let bound_order = compare_decimal_strings(&interval.lower_decimal, &interval.upper_decimal)
        .ok_or_else(|| "reference interval endpoints must be finite plain decimals".to_owned())?;
    if bound_order == std::cmp::Ordering::Greater {
        return Err("reference interval lower bound exceeds its upper bound".to_owned());
    }
    let lower_order = compare_decimal_strings(&candidate.value_decimal, &interval.lower_decimal)
        .ok_or_else(|| "candidate value and interval lower bound must be finite plain decimals".to_owned())?;
    let upper_order = compare_decimal_strings(&candidate.value_decimal, &interval.upper_decimal)
        .ok_or_else(|| "candidate value and interval upper bound must be finite plain decimals".to_owned())?;

    let (position, distance_to_interval_decimal) = if lower_order == std::cmp::Ordering::Less {
        (
            IntervalPosition::Below,
            subtract_decimal_strings(&interval.lower_decimal, &candidate.value_decimal)
                .ok_or_else(|| "could not calculate exact decimal distance".to_owned())?,
        )
    } else if upper_order == std::cmp::Ordering::Greater {
        (
            IntervalPosition::Above,
            subtract_decimal_strings(&candidate.value_decimal, &interval.upper_decimal)
                .ok_or_else(|| "could not calculate exact decimal distance".to_owned())?,
        )
    } else {
        (IntervalPosition::Within, "0".to_owned())
    };

    Ok(PointIntervalDiagnostic {
        contrast_id: contrast.id.clone(),
        candidate_estimate_id: candidate.id.clone(),
        reference_estimate_id: reference.id.clone(),
        quantity: candidate.quantity.clone(),
        unit: candidate.unit.clone(),
        candidate_value_decimal: candidate.value_decimal.clone(),
        interval_lower_decimal: interval.lower_decimal.clone(),
        interval_upper_decimal: interval.upper_decimal.clone(),
        interval_kind: interval.kind,
        coverage_percent: interval.coverage_percent.clone(),
        position,
        distance_to_interval_decimal,
        interpretation_limit: contrast.interpretation_limit.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelComparison {
    pub id: String,
    pub question: String,
    pub model_ids: Vec<String>,
    pub evaluation_axes: Vec<String>,
    /// Declares whether the comparison avoids a single aggregate winner or uses a defined rule.
    pub decision_rule: String,
    pub caveats: Vec<String>,
    /// Evaluations are explicit; an axis without an assessment is not implied to have passed.
    #[serde(default)]
    pub assessments: Vec<ModelAssessment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAssessment {
    pub model_id: String,
    pub axis: String,
    pub status: AssessmentStatus,
    pub summary: String,
    #[serde(default)]
    pub estimate_ids: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<EvidenceLink>,
    #[serde(default)]
    pub interpretation_note: Option<String>,
    /// Required for independently reproduced results; identify the versioned run/test manifest.
    #[serde(default)]
    pub validation_artifact: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    SourceReported,
    IndependentlyReproduced,
    Hypothesis,
    NotEvaluated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCode {
    EmptySchemaVersion,
    EmptyId,
    DuplicateId,
    EmptyField,
    InvalidTimeInterval,
    MissingEvidenceForClaim,
    MissingSourceReference,
    MissingEvidenceLocator,
    MissingClaimReference,
    MissingEvidenceForRelation,
    MissingInterpretationNote,
    InvalidRightsDeclaration,
    UndistributableRights,
    InvalidSha256,
    SelfRelation,
    TemporalContradiction,
    UnsupportedSchemaVersion,
    InvalidNumericEstimate,
    InvalidUncertaintyInterval,
    MissingEstimateEvidence,
    MissingModelReference,
    MissingEstimateReference,
    InvalidModelComparison,
    InvalidModelAssessment,
    InvalidEstimateContrast,
    InvalidEstimateDelta,
    InvalidAnalysisRun,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub record_id: String,
    pub code: ValidationCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.issues.len()
    }
}

fn issue(
    issues: &mut Vec<ValidationIssue>,
    record_id: impl Into<String>,
    code: ValidationCode,
    message: impl Into<String>,
) {
    issues.push(ValidationIssue {
        record_id: record_id.into(),
        code,
        message: message.into(),
    });
}

fn register_id(
    id: &str,
    seen: &mut BTreeSet<String>,
    record_id: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if id.trim().is_empty() {
        issue(
            issues,
            if record_id.is_empty() { "<empty>" } else { record_id },
            ValidationCode::EmptyId,
            "record ID must not be empty",
        );
    } else if !seen.insert(id.to_owned()) {
        issue(
            issues,
            id,
            ValidationCode::DuplicateId,
            "record ID is already used by another record",
        );
    }
}

fn require_nonempty(
    value: &str,
    record_id: &str,
    field: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if value.trim().is_empty() {
        issue(
            issues,
            record_id,
            ValidationCode::EmptyField,
            format!("{field} must not be empty"),
        );
    }
}

fn validate_rights(
    rights: &RightsDeclaration,
    record_id: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    match rights.status {
        RightsStatus::OpenLicense
            if rights.license_id.as_deref().unwrap_or("").trim().is_empty() =>
        {
            issue(
                issues,
                record_id,
                ValidationCode::InvalidRightsDeclaration,
                "open-license status requires an exact license identifier",
            );
        }
        RightsStatus::PermissionGranted
            if rights.basis_note.as_deref().unwrap_or("").trim().is_empty() =>
        {
            issue(
                issues,
                record_id,
                ValidationCode::InvalidRightsDeclaration,
                "permission-granted status requires a recorded permission basis",
            );
        }
        RightsStatus::PublicDomain
            if rights.basis_note.as_deref().unwrap_or("").trim().is_empty() =>
        {
            issue(
                issues,
                record_id,
                ValidationCode::InvalidRightsDeclaration,
                "public-domain status requires a documented determination basis",
            );
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy)]
struct DecimalParts {
    coefficient: i128,
    scale: u32,
}

/// Parse a plain base-10 decimal exactly. Exponents and non-finite tokens are rejected.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_decimal_parts(value: &str) -> Option<DecimalParts> {
    if value.trim() != value || value.is_empty() {
        return None;
    }
    let (negative, unsigned) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };
    let mut parts = unsigned.split('.');
    let whole = parts.next()?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some() || (whole.is_empty() && fraction.is_empty()) {
        return None;
    }
    if (!whole.is_empty() && !whole.bytes().all(|byte| byte.is_ascii_digit()))
        || (!fraction.is_empty() && !fraction.bytes().all(|byte| byte.is_ascii_digit()))
        || fraction.len() > 18
    {
        return None;
    }
    let whole = if whole.is_empty() { "0" } else { whole };
    let digits = format!("{whole}{fraction}");
    let mut coefficient = digits.parse::<i128>().ok()?;
    if negative {
        coefficient = coefficient.checked_neg()?;
    }
    let mut scale = fraction.len() as u32;
    while scale > 0 && coefficient % 10 == 0 {
        coefficient /= 10;
        scale -= 1;
    }
    Some(DecimalParts { coefficient, scale })
}

fn decimal_power10(power: u32) -> Option<i128> {
    (0..power).try_fold(1_i128, |value, _| value.checked_mul(10))
}

fn aligned_decimal_coefficients(left: &str, right: &str) -> Option<(i128, i128, u32)> {
    let left = parse_decimal_parts(left)?;
    let right = parse_decimal_parts(right)?;
    let scale = left.scale.max(right.scale);
    let left_factor = decimal_power10(scale - left.scale)?;
    let right_factor = decimal_power10(scale - right.scale)?;
    Some((
        left.coefficient.checked_mul(left_factor)?,
        right.coefficient.checked_mul(right_factor)?,
        scale,
    ))
}

fn compare_decimal_strings(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let (left, right, _) = aligned_decimal_coefficients(left, right)?;
    Some(left.cmp(&right))
}

fn format_decimal(coefficient: i128, scale: u32) -> String {
    let negative = coefficient < 0;
    let digits = coefficient.unsigned_abs().to_string();
    let sign = if negative && coefficient != 0 { "-" } else { "" };
    if scale == 0 {
        return format!("{sign}{digits}");
    }
    let padded = if digits.len() <= scale as usize {
        format!("{}{}", "0".repeat(scale as usize + 1 - digits.len()), digits)
    } else {
        digits
    };
    let split = padded.len() - scale as usize;
    let whole = &padded[..split];
    let fraction = padded[split..].trim_end_matches('0');
    if fraction.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction}")
    }
}

fn subtract_decimal_strings(left: &str, right: &str) -> Option<String> {
    let (left, right, scale) = aligned_decimal_coefficients(left, right)?;
    Some(format_decimal(left.checked_sub(right)?, scale))
}

fn parse_finite_decimal(value: &str) -> Option<f64> {
    let decimal = parse_decimal_parts(value)?;
    let denominator = decimal_power10(decimal.scale)? as f64;
    let number = decimal.coefficient as f64 / denominator;
    number.is_finite().then_some(number)
}

fn is_finite_decimal(value: &str) -> bool {
    parse_decimal_parts(value).is_some()
}

fn validate_evidence_links(
    owner_id: &str,
    evidence: &[EvidenceLink],
    source_ids: &BTreeSet<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    for link in evidence {
        if !source_ids.contains(&link.source_id) {
            issue(
                issues,
                owner_id,
                ValidationCode::MissingSourceReference,
                format!("evidence refers to unknown source ID '{}'", link.source_id),
            );
        }
        require_nonempty(&link.locator, owner_id, "evidence locator", issues);
    }
}

/// Validate IDs, references, claim modes, chronology intervals, rights, and provenance invariants.
///
/// This is a structural validator, not an automated historian: it cannot determine whether a
/// source is credible, whether an interpretation is correct, or whether cited evidence actually
/// entails its claim. Those require domain review and separate benchmark protocols.
pub fn validate(dataset: &Dataset) -> ValidationReport {
    let mut issues = Vec::new();
    let mut all_ids = BTreeSet::new();

    if dataset.schema_version.trim().is_empty() {
        issue(
            &mut issues,
            "dataset",
            ValidationCode::EmptySchemaVersion,
            "dataset schema version is required",
        );
    } else if dataset.schema_version != SCHEMA_VERSION {
        issue(
            &mut issues,
            "dataset",
            ValidationCode::UnsupportedSchemaVersion,
            format!(
                "schema version '{}' is not supported; expected '{}'",
                dataset.schema_version, SCHEMA_VERSION
            ),
        );
    }

    for source in &dataset.sources {
        register_id(&source.id, &mut all_ids, &source.id, &mut issues);
        require_nonempty(&source.title, &source.id, "source title", &mut issues);
        require_nonempty(&source.source_uri, &source.id, "source URI/identifier", &mut issues);
        validate_rights(&source.rights, &source.id, &mut issues);
    }

    let source_ids: BTreeSet<String> = dataset.sources.iter().map(|source| source.id.clone()).collect();
    let claim_ids: BTreeSet<String> = dataset.claims.iter().map(|claim| claim.id.clone()).collect();

    for claim in &dataset.claims {
        register_id(&claim.id, &mut all_ids, &claim.id, &mut issues);
        require_nonempty(&claim.proposition, &claim.id, "claim proposition", &mut issues);

        if claim.mode.requires_evidence()
            && !claim
                .evidence
                .iter()
                .any(|link| link.relation == EvidenceRelation::Supports)
        {
            issue(
                &mut issues,
                &claim.id,
                ValidationCode::MissingEvidenceForClaim,
                "attested claims and evidence-based reconstructions require at least one evidence link marked supports; challenging-only evidence is not positive support",
            );
        }
        if claim.mode.requires_interpretation_note()
            && claim.interpretation_note.as_deref().unwrap_or("").trim().is_empty()
        {
            issue(
                &mut issues,
                &claim.id,
                ValidationCode::MissingInterpretationNote,
                "hypotheses and counterfactuals require an interpretation note",
            );
        }

        if let Some(time) = &claim.valid_time {
            require_nonempty(&time.calendar, &claim.id, "time calendar", &mut issues);
            if time.start_year > time.end_year {
                issue(
                    &mut issues,
                    &claim.id,
                    ValidationCode::InvalidTimeInterval,
                    "time interval start_year must not exceed end_year in the declared calendar",
                );
            } else if time.precision == DatePrecision::ExactYear
                && time.start_year != time.end_year
            {
                issue(
                    &mut issues,
                    &claim.id,
                    ValidationCode::InvalidTimeInterval,
                    "exact_year precision requires identical start_year and end_year",
                );
            }
        }
        validate_evidence_links(&claim.id, &claim.evidence, &source_ids, &mut issues);
    }

    for relation in &dataset.relations {
        register_id(&relation.id, &mut all_ids, &relation.id, &mut issues);
        if !claim_ids.contains(&relation.from_claim_id) {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingClaimReference,
                format!("relation refers to unknown from_claim_id '{}'", relation.from_claim_id),
            );
        }
        if !claim_ids.contains(&relation.to_claim_id) {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingClaimReference,
                format!("relation refers to unknown to_claim_id '{}'", relation.to_claim_id),
            );
        }
        if !relation.from_claim_id.is_empty() && relation.from_claim_id == relation.to_claim_id {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::SelfRelation,
                "a relation must not connect a claim to itself",
            );
        }

        // Detect only contradictions that are decidable from intervals in the same calendar.
        // Overlapping/uncertain intervals are not treated as proof that an edge is false.
        let time_directed = matches!(
            relation.kind,
            RelationKind::Precedes
                | RelationKind::Influenced
                | RelationKind::TransmittedVia
                | RelationKind::DerivedFrom
                | RelationKind::Enabled
        );
        if time_directed {
            let from_time = dataset
                .claims
                .iter()
                .find(|claim| claim.id == relation.from_claim_id)
                .and_then(|claim| claim.valid_time.as_ref());
            let to_time = dataset
                .claims
                .iter()
                .find(|claim| claim.id == relation.to_claim_id)
                .and_then(|claim| claim.valid_time.as_ref());
            if let (Some(from), Some(to)) = (from_time, to_time) {
                if from.calendar == to.calendar && from.start_year > to.end_year {
                    issue(
                        &mut issues,
                        &relation.id,
                        ValidationCode::TemporalContradiction,
                        "directed temporal/influence relation conflicts with the non-overlapping intervals in the same calendar",
                    );
                }
            }
        }

        if relation.status == RelationStatus::Documented
            && !relation
                .evidence
                .iter()
                .any(|link| link.relation == EvidenceRelation::Supports)
        {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingEvidenceForRelation,
                "documented relationships require at least one evidence link marked supports; challenging-only evidence cannot document the edge",
            );
        }
        if matches!(
            relation.status,
            RelationStatus::Hypothesized | RelationStatus::Counterfactual
        ) && relation.interpretation_note.as_deref().unwrap_or("").trim().is_empty()
        {
            issue(
                &mut issues,
                &relation.id,
                ValidationCode::MissingInterpretationNote,
                "hypothesized and counterfactual relationships require an interpretation note",
            );
        }
        validate_evidence_links(&relation.id, &relation.evidence, &source_ids, &mut issues);
    }

    for asset in &dataset.assets {
        register_id(&asset.id, &mut all_ids, &asset.id, &mut issues);
        if !source_ids.contains(&asset.source_id) {
            issue(
                &mut issues,
                &asset.id,
                ValidationCode::MissingSourceReference,
                format!("asset refers to unknown provenance source ID '{}'", asset.source_id),
            );
        }
        validate_rights(&asset.rights, &asset.id, &mut issues);
        if asset.content_sha256.len() != 64
            || !asset.content_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            issue(
                &mut issues,
                &asset.id,
                ValidationCode::InvalidSha256,
                "asset content_sha256 must contain exactly 64 hexadecimal characters",
            );
        }
        if asset.distributable
            && matches!(
                asset.rights.status,
                RightsStatus::Restricted | RightsStatus::Unknown | RightsStatus::CommunityProtocol
            )
        {
            issue(
                &mut issues,
                &asset.id,
                ValidationCode::UndistributableRights,
                "asset marked distributable has restricted, unknown, or community-protocol rights",
            );
        }
    }

    for estimate in &dataset.estimates {
        register_id(&estimate.id, &mut all_ids, &estimate.id, &mut issues);
        if !claim_ids.contains(&estimate.claim_id) {
            issue(
                &mut issues,
                &estimate.id,
                ValidationCode::MissingClaimReference,
                format!("estimate refers to unknown claim ID '{}'", estimate.claim_id),
            );
        }
        require_nonempty(&estimate.quantity, &estimate.id, "estimate quantity", &mut issues);
        require_nonempty(&estimate.unit, &estimate.id, "estimate unit", &mut issues);
        require_nonempty(&estimate.method, &estimate.id, "estimate method", &mut issues);
        if estimate.assumptions.is_empty()
            || estimate.assumptions.iter().any(|assumption| assumption.trim().is_empty())
        {
            issue(
                &mut issues,
                &estimate.id,
                ValidationCode::InvalidNumericEstimate,
                "estimates require explicit assumptions; state that no assumptions are supplied when appropriate",
            );
        }

        if !is_finite_decimal(&estimate.value_decimal) {
            issue(
                &mut issues,
                &estimate.id,
                ValidationCode::InvalidNumericEstimate,
                "value_decimal must be a finite decimal number encoded as a string",
            );
        }

        if estimate.interval.is_none()
            && estimate.kind == EstimateKind::ReportedIntervalEstimate
        {
            issue(
                &mut issues,
                &estimate.id,
                ValidationCode::InvalidUncertaintyInterval,
                "reported_interval_estimate requires a declared uncertainty interval",
            );
        }

        if let Some(interval) = &estimate.interval {
            let interval_order = compare_decimal_strings(
                &interval.lower_decimal,
                &interval.upper_decimal,
            );
            if interval_order.is_none()
                || interval_order == Some(std::cmp::Ordering::Greater)
            {
                issue(
                    &mut issues,
                    &estimate.id,
                    ValidationCode::InvalidUncertaintyInterval,
                    "uncertainty interval endpoints must be finite plain decimals with lower <= upper",
                );
            }
            require_nonempty(
                &interval.conditioning_note,
                &estimate.id,
                "interval conditioning note",
                &mut issues,
            );

            match interval.coverage_percent.as_deref() {
                Some(coverage) => match parse_finite_decimal(coverage) {
                    Some(value) if value > 0.0 && value <= 100.0 => {}
                    _ => issue(
                        &mut issues,
                        &estimate.id,
                        ValidationCode::InvalidUncertaintyInterval,
                        "coverage_percent must be greater than 0 and at most 100",
                    ),
                },
                None if matches!(interval.kind, IntervalKind::Credible | IntervalKind::Confidence) => {
                    issue(
                        &mut issues,
                        &estimate.id,
                        ValidationCode::InvalidUncertaintyInterval,
                        "credible and confidence intervals require an explicit coverage_percent",
                    );
                }
                None => {}
            }
        }

        if !estimate.evidence.iter().any(|link| link.relation == EvidenceRelation::Supports) {
            issue(
                &mut issues,
                &estimate.id,
                ValidationCode::MissingEstimateEvidence,
                "reported estimates and reconstruction parameters require a source link marked supports",
            );
        }
        validate_evidence_links(&estimate.id, &estimate.evidence, &source_ids, &mut issues);
    }

    let estimate_ids: BTreeSet<String> =
        dataset.estimates.iter().map(|estimate| estimate.id.clone()).collect();

    for contrast in &dataset.estimate_contrasts {
        register_id(&contrast.id, &mut all_ids, &contrast.id, &mut issues);
        require_nonempty(&contrast.purpose, &contrast.id, "contrast purpose", &mut issues);
        require_nonempty(&contrast.dependence_note, &contrast.id, "contrast dependence note", &mut issues);
        require_nonempty(
            &contrast.interpretation_limit,
            &contrast.id,
            "contrast interpretation limit",
            &mut issues,
        );
        if !estimate_ids.contains(&contrast.candidate_estimate_id) {
            issue(
                &mut issues,
                &contrast.id,
                ValidationCode::InvalidEstimateContrast,
                format!("contrast refers to unknown candidate estimate '{}'", contrast.candidate_estimate_id),
            );
        }
        if !estimate_ids.contains(&contrast.reference_estimate_id) {
            issue(
                &mut issues,
                &contrast.id,
                ValidationCode::InvalidEstimateContrast,
                format!("contrast refers to unknown reference estimate '{}'", contrast.reference_estimate_id),
            );
        }
        let candidate = dataset.estimates.iter().find(|e| e.id == contrast.candidate_estimate_id);
        let reference = dataset.estimates.iter().find(|e| e.id == contrast.reference_estimate_id);
        if let (Some(candidate), Some(reference)) = (candidate, reference) {
            if candidate.id == reference.id {
                issue(
                    &mut issues,
                    &contrast.id,
                    ValidationCode::InvalidEstimateContrast,
                    "candidate and reference estimates must be different records",
                );
            }
            if candidate.quantity != reference.quantity || candidate.unit != reference.unit {
                issue(
                    &mut issues,
                    &contrast.id,
                    ValidationCode::InvalidEstimateContrast,
                    "candidate and reference must describe the same quantity and unit",
                );
            }
            if candidate.interval.is_some() {
                issue(
                    &mut issues,
                    &contrast.id,
                    ValidationCode::InvalidEstimateContrast,
                    "candidate estimate must be point-valued",
                );
            }
            if reference.interval.is_none() {
                issue(
                    &mut issues,
                    &contrast.id,
                    ValidationCode::InvalidEstimateContrast,
                    "reference estimate must have a reported uncertainty interval",
                );
            }
        }
    }

    for contrast in &dataset.estimate_contrasts {
        require_nonempty(&contrast.purpose, &contrast.id, "contrast purpose", &mut issues);
        require_nonempty(&contrast.dependence_note, &contrast.id, "contrast dependence note", &mut issues);
        require_nonempty(
            &contrast.interpretation_limit,
            &contrast.id,
            "contrast interpretation limit",
            &mut issues,
        );
        let candidate = dataset.estimates.iter().find(|e| e.id == contrast.candidate_estimate_id);
        let reference = dataset.estimates.iter().find(|e| e.id == contrast.reference_estimate_id);
        match (candidate, reference) {
            (None, _) => issue(
                &mut issues,
                &contrast.id,
                ValidationCode::InvalidEstimateContrast,
                format!("contrast refers to unknown candidate estimate '{}'", contrast.candidate_estimate_id),
            ),
            (_, None) => issue(
                &mut issues,
                &contrast.id,
                ValidationCode::InvalidEstimateContrast,
                format!("contrast refers to unknown reference estimate '{}'", contrast.reference_estimate_id),
            ),
            (Some(candidate), Some(reference)) => {
                if candidate.id == reference.id {
                    issue(
                        &mut issues,
                        &contrast.id,
                        ValidationCode::InvalidEstimateContrast,
                        "candidate and reference estimates must be different records",
                    );
                }
                if candidate.quantity != reference.quantity || candidate.unit != reference.unit {
                    issue(
                        &mut issues,
                        &contrast.id,
                        ValidationCode::InvalidEstimateContrast,
                        "candidate and reference must describe the same quantity and unit",
                    );
                }
                if candidate.interval.is_some() {
                    issue(
                        &mut issues,
                        &contrast.id,
                        ValidationCode::InvalidEstimateContrast,
                        "candidate estimate must be point-valued",
                    );
                }
                if reference.interval.is_none() {
                    issue(
                        &mut issues,
                        &contrast.id,
                        ValidationCode::InvalidEstimateContrast,
                        "reference estimate must have a reported uncertainty interval",
                    );
                }
            }
        }
    }

    for delta in &dataset.estimate_deltas {
        register_id(&delta.id, &mut all_ids, &delta.id, &mut issues);
        require_nonempty(&delta.variant_description, &delta.id, "variant description", &mut issues);
        require_nonempty(&delta.dependence_note, &delta.id, "delta dependence note", &mut issues);
        require_nonempty(
            &delta.interpretation_limit,
            &delta.id,
            "delta interpretation limit",
            &mut issues,
        );
        let baseline = dataset.estimates.iter().find(|e| e.id == delta.baseline_estimate_id);
        let perturbed = dataset.estimates.iter().find(|e| e.id == delta.perturbed_estimate_id);
        match (baseline, perturbed) {
            (None, _) => issue(
                &mut issues,
                &delta.id,
                ValidationCode::InvalidEstimateDelta,
                format!("delta refers to unknown baseline estimate '{}'", delta.baseline_estimate_id),
            ),
            (_, None) => issue(
                &mut issues,
                &delta.id,
                ValidationCode::InvalidEstimateDelta,
                format!("delta refers to unknown perturbed estimate '{}'", delta.perturbed_estimate_id),
            ),
            (Some(baseline), Some(perturbed)) => {
                if baseline.id == perturbed.id {
                    issue(
                        &mut issues,
                        &delta.id,
                        ValidationCode::InvalidEstimateDelta,
                        "baseline and perturbed estimates must be different records",
                    );
                }
                if baseline.quantity != perturbed.quantity || baseline.unit != perturbed.unit {
                    issue(
                        &mut issues,
                        &delta.id,
                        ValidationCode::InvalidEstimateDelta,
                        "baseline and perturbed estimates must describe the same quantity and unit",
                    );
                }
                if baseline.interval.is_some() || perturbed.interval.is_some() {
                    issue(
                        &mut issues,
                        &delta.id,
                        ValidationCode::InvalidEstimateDelta,
                        "paired estimate delta requires two point-valued estimates",
                    );
                }
            }
        }
    }

    for run in &dataset.analysis_runs {
        register_id(&run.id, &mut all_ids, &run.id, &mut issues);
        require_nonempty(&run.purpose, &run.id, "analysis run purpose", &mut issues);
        require_nonempty(&run.model_spec_version, &run.id, "model specification version", &mut issues);
        if run.limitations.is_empty() || run.limitations.iter().any(|limitation| limitation.trim().is_empty()) {
            issue(
                &mut issues,
                &run.id,
                ValidationCode::InvalidAnalysisRun,
                "analysis run must record at least one explicit limitation",
            );
        }

        let requires_execution_record = matches!(
            run.status,
            AnalysisRunStatus::Executed | AnalysisRunStatus::Reproduced | AnalysisRunStatus::Failed
        );
        let requires_inputs = matches!(
            run.status,
            AnalysisRunStatus::InputsVerified
                | AnalysisRunStatus::Executed
                | AnalysisRunStatus::Reproduced
                | AnalysisRunStatus::Failed
        );

        if requires_inputs {
            if run.input_artifacts.is_empty() {
                issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::InvalidAnalysisRun,
                    "input-verified or later runs require at least one content-addressed input artifact",
                );
            }
            match run.input_manifest_sha256.as_deref() {
                Some(digest) if is_sha256(digest) => {}
                _ => issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::InvalidAnalysisRun,
                    "input-verified or later runs require a valid input_manifest_sha256",
                ),
            }
        }

        if requires_execution_record {
            require_nonempty(
                run.command.as_deref().unwrap_or(""),
                &run.id,
                "executed command",
                &mut issues,
            );
            require_nonempty(
                run.code_revision.as_deref().unwrap_or(""),
                &run.id,
                "code revision",
                &mut issues,
            );
            require_nonempty(
                run.environment_fingerprint.as_deref().unwrap_or(""),
                &run.id,
                "environment fingerprint",
                &mut issues,
            );
            require_nonempty(
                run.result_summary.as_deref().unwrap_or(""),
                &run.id,
                "result or failure summary",
                &mut issues,
            );
        }

        if matches!(run.status, AnalysisRunStatus::Executed | AnalysisRunStatus::Reproduced)
            && run.output_artifacts.is_empty()
        {
            issue(
                &mut issues,
                &run.id,
                ValidationCode::InvalidAnalysisRun,
                "executed or reproduced runs require at least one content-addressed output artifact",
            );
        }

        if run.status == AnalysisRunStatus::Reproduced {
            if !run
                .comparison_source_id
                .as_deref()
                .is_some_and(|source_id| source_ids.contains(source_id))
            {
                issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::InvalidAnalysisRun,
                    "reproduced runs require a comparison_source_id pointing to a source record",
                );
            }
            if run.target_estimate_ids.is_empty() {
                issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::InvalidAnalysisRun,
                    "reproduced runs must identify the target estimate records being reproduced",
                );
            }
            require_nonempty(
                run.comparison_rule.as_deref().unwrap_or(""),
                &run.id,
                "preregistered comparison rule",
                &mut issues,
            );
        }

        for target_id in &run.target_estimate_ids {
            if !estimate_ids.contains(target_id) {
                issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::MissingEstimateReference,
                    format!("analysis run refers to unknown target estimate ID '{target_id}'"),
                );
            }
        }

        for artifact in run.input_artifacts.iter().chain(run.output_artifacts.iter()) {
            require_nonempty(&artifact.name, &run.id, "run artifact name", &mut issues);
            if !is_sha256(&artifact.sha256) {
                issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::InvalidAnalysisRun,
                    format!("run artifact '{}' must have a 64-character SHA-256 digest", artifact.name),
                );
            }
        }

        let mut input_names = BTreeSet::new();
        for artifact in &run.input_artifacts {
            if !input_names.insert(artifact.name.as_str()) {
                issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::InvalidAnalysisRun,
                    format!("duplicate input artifact name '{}'", artifact.name),
                );
            }
        }
        let mut output_names = BTreeSet::new();
        for artifact in &run.output_artifacts {
            if !output_names.insert(artifact.name.as_str()) {
                issue(
                    &mut issues,
                    &run.id,
                    ValidationCode::InvalidAnalysisRun,
                    format!("duplicate output artifact name '{}'", artifact.name),
                );
            }
        }
    }

    for model in &dataset.reconstruction_models {
        register_id(&model.id, &mut all_ids, &model.id, &mut issues);
        require_nonempty(&model.title, &model.id, "model title", &mut issues);
        require_nonempty(&model.summary, &model.id, "model summary", &mut issues);
        require_nonempty(&model.method, &model.id, "model method", &mut issues);
        if model.target_claim_ids.is_empty() || model.estimate_ids.is_empty() {
            issue(
                &mut issues,
                &model.id,
                ValidationCode::MissingModelReference,
                "a reconstruction model must reference at least one target claim and one quantitative estimate",
            );
        }
        if model.assumptions.is_empty()
            || model.assumptions.iter().any(|assumption| assumption.trim().is_empty())
        {
            issue(
                &mut issues,
                &model.id,
                ValidationCode::InvalidModelComparison,
                "reconstruction models require explicit assumptions; state what remains unspecified",
            );
        }
        for claim_id in &model.target_claim_ids {
            if !claim_ids.contains(claim_id) {
                issue(
                    &mut issues,
                    &model.id,
                    ValidationCode::MissingClaimReference,
                    format!("model refers to unknown target claim ID '{claim_id}'"),
                );
            }
        }
        for estimate_id in &model.estimate_ids {
            if !estimate_ids.contains(estimate_id) {
                issue(
                    &mut issues,
                    &model.id,
                    ValidationCode::MissingEstimateReference,
                    format!("model refers to unknown estimate ID '{estimate_id}'"),
                );
            }
        }
        if !model.evidence.iter().any(|link| link.relation == EvidenceRelation::Supports) {
            issue(
                &mut issues,
                &model.id,
                ValidationCode::MissingEstimateEvidence,
                "a published interpretation or candidate model requires source-linked supporting evidence",
            );
        }
        validate_evidence_links(&model.id, &model.evidence, &source_ids, &mut issues);
    }

    let model_ids: BTreeSet<String> = dataset
        .reconstruction_models
        .iter()
        .map(|model| model.id.clone())
        .collect();

    for comparison in &dataset.model_comparisons {
        register_id(&comparison.id, &mut all_ids, &comparison.id, &mut issues);
        require_nonempty(&comparison.question, &comparison.id, "comparison question", &mut issues);
        require_nonempty(&comparison.decision_rule, &comparison.id, "comparison decision rule", &mut issues);
        let unique_models: BTreeSet<&String> = comparison.model_ids.iter().collect();
        if unique_models.len() < 2 || unique_models.len() != comparison.model_ids.len() {
            issue(
                &mut issues,
                &comparison.id,
                ValidationCode::InvalidModelComparison,
                "a model comparison must reference at least two distinct models",
            );
        }
        if comparison.evaluation_axes.is_empty()
            || comparison.evaluation_axes.iter().any(|axis| axis.trim().is_empty())
        {
            issue(
                &mut issues,
                &comparison.id,
                ValidationCode::InvalidModelComparison,
                "a model comparison requires named evaluation axes",
            );
        }
        for model_id in &comparison.model_ids {
            if !model_ids.contains(model_id) {
                issue(
                    &mut issues,
                    &comparison.id,
                    ValidationCode::MissingModelReference,
                    format!("comparison refers to unknown model ID '{model_id}'"),
                );
            }
        }
        if comparison.caveats.iter().any(|caveat| caveat.trim().is_empty()) {
            issue(
                &mut issues,
                &comparison.id,
                ValidationCode::InvalidModelComparison,
                "comparison caveats must not contain empty strings",
            );
        }

        let mut assessment_keys = BTreeSet::new();
        for assessment in &comparison.assessments {
            if !comparison.model_ids.contains(&assessment.model_id)
                || !model_ids.contains(&assessment.model_id)
            {
                issue(
                    &mut issues,
                    &comparison.id,
                    ValidationCode::InvalidModelAssessment,
                    format!("assessment refers to model '{}' outside this comparison", assessment.model_id),
                );
            }
            if !comparison.evaluation_axes.contains(&assessment.axis) {
                issue(
                    &mut issues,
                    &comparison.id,
                    ValidationCode::InvalidModelAssessment,
                    format!("assessment uses undeclared evaluation axis '{}'", assessment.axis),
                );
            }
            if !assessment_keys.insert((assessment.model_id.as_str(), assessment.axis.as_str())) {
                issue(
                    &mut issues,
                    &comparison.id,
                    ValidationCode::InvalidModelAssessment,
                    format!("duplicate assessment for model '{}' on axis '{}'", assessment.model_id, assessment.axis),
                );
            }
            require_nonempty(&assessment.summary, &comparison.id, "assessment summary", &mut issues);
            for estimate_id in &assessment.estimate_ids {
                if !estimate_ids.contains(estimate_id) {
                    issue(
                        &mut issues,
                        &comparison.id,
                        ValidationCode::MissingEstimateReference,
                        format!("assessment refers to unknown estimate ID '{estimate_id}'"),
                    );
                }
            }

            match assessment.status {
                AssessmentStatus::SourceReported => {
                    if !assessment.evidence.iter().any(|link| link.relation == EvidenceRelation::Supports) {
                        issue(
                            &mut issues,
                            &comparison.id,
                            ValidationCode::InvalidModelAssessment,
                            "source_reported assessments require supporting evidence",
                        );
                    }
                }
                AssessmentStatus::IndependentlyReproduced => {
                    if !assessment.evidence.iter().any(|link| link.relation == EvidenceRelation::Supports)
                        || assessment.validation_artifact.as_deref().unwrap_or("").trim().is_empty()
                    {
                        issue(
                            &mut issues,
                            &comparison.id,
                            ValidationCode::InvalidModelAssessment,
                            "independently_reproduced assessments require supporting evidence and a versioned validation artifact",
                        );
                    }
                }
                AssessmentStatus::Hypothesis => {
                    if assessment.interpretation_note.as_deref().unwrap_or("").trim().is_empty() {
                        issue(
                            &mut issues,
                            &comparison.id,
                            ValidationCode::InvalidModelAssessment,
                            "hypothesis assessments require an explicit interpretation note",
                        );
                    }
                }
                AssessmentStatus::NotEvaluated => {}
            }
            validate_evidence_links(&comparison.id, &assessment.evidence, &source_ids, &mut issues);
        }
    }

    // Report order is stable regardless of source/vector insertion order.
    issues.sort();
    ValidationReport { issues }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rights_open() -> RightsDeclaration {
        RightsDeclaration {
            status: RightsStatus::OpenLicense,
            license_id: Some("CC0-1.0".to_owned()),
            attribution: None,
            basis_note: None,
        }
    }

    fn evidence(source_id: &str) -> EvidenceLink {
        EvidenceLink {
            source_id: source_id.to_owned(),
            locator: "fixture:paragraph-1".to_owned(),
            relation: EvidenceRelation::Supports,
            note: None,
        }
    }

    fn estimate_for_fixture(id: &str, claim_id: &str) -> QuantitativeEstimate {
        QuantitativeEstimate {
            id: id.to_owned(),
            claim_id: claim_id.to_owned(),
            quantity: "synthetic count".to_owned(),
            value_decimal: "1.0".to_owned(),
            unit: "count".to_owned(),
            kind: EstimateKind::ReportedIntervalEstimate,
            method: "synthetic interval method".to_owned(),
            assumptions: vec!["Synthetic values; not a historical measurement".to_owned()],
            interval: Some(UncertaintyInterval {
                lower_decimal: "0.8".to_owned(),
                upper_decimal: "1.2".to_owned(),
                kind: IntervalKind::Credible,
                coverage_percent: Some("68".to_owned()),
                conditioning_note: "Synthetic fixture interval".to_owned(),
            }),
            evidence: vec![evidence("fixture:source-1")],
        }
    }

    fn model_for_fixture(id: &str, claim_id: &str, estimate_id: &str) -> ReconstructionModel {
        ReconstructionModel {
            id: id.to_owned(),
            title: format!("Synthetic model {id}"),
            summary: "Synthetic model for validator tests only".to_owned(),
            status: ReconstructionModelStatus::PublishedInterpretation,
            method: "synthetic model method".to_owned(),
            target_claim_ids: vec![claim_id.to_owned()],
            estimate_ids: vec![estimate_id.to_owned()],
            assumptions: vec!["Synthetic and not a historical assertion".to_owned()],
            evidence: vec![evidence("fixture:source-1")],
        }
    }

    fn run_for_fixture(status: AnalysisRunStatus) -> AnalysisRun {
        let executed = matches!(
            status,
            AnalysisRunStatus::Executed | AnalysisRunStatus::Reproduced | AnalysisRunStatus::Failed
        );
        let uses_inputs = matches!(
            status,
            AnalysisRunStatus::InputsVerified
                | AnalysisRunStatus::Executed
                | AnalysisRunStatus::Reproduced
                | AnalysisRunStatus::Failed
        );
        AnalysisRun {
            id: "fixture:run-1".to_owned(),
            purpose: "Synthetic run for validation tests only".to_owned(),
            status,
            model_spec_version: "synthetic-model-v1".to_owned(),
            code_revision: executed.then(|| "0123456789abcdef0123456789abcdef01234567".to_owned()),
            command: executed.then(|| "synthetic-test-command".to_owned()),
            environment_fingerprint: executed.then(|| "nix-lock:synthetic-fixture".to_owned()),
            input_manifest_sha256: uses_inputs.then(|| "a".repeat(64)),
            input_artifacts: if uses_inputs {
                vec![RunArtifactDigest {
                    name: "synthetic-input.json".to_owned(),
                    sha256: "b".repeat(64),
                }]
            } else {
                vec![]
            },
            output_artifacts: if matches!(status, AnalysisRunStatus::Executed | AnalysisRunStatus::Reproduced) {
                vec![RunArtifactDigest {
                    name: "synthetic-output.json".to_owned(),
                    sha256: "c".repeat(64),
                }]
            } else {
                vec![]
            },
            random_seed: Some("seed-42".to_owned()),
            target_estimate_ids: if status == AnalysisRunStatus::Reproduced {
                vec!["fixture:estimate-a".to_owned()]
            } else {
                vec![]
            },
            comparison_source_id: (status == AnalysisRunStatus::Reproduced)
                .then(|| "fixture:source-1".to_owned()),
            comparison_rule: (status == AnalysisRunStatus::Reproduced)
                .then(|| "Synthetic comparison rule".to_owned()),
            result_summary: executed.then(|| "Synthetic run result only".to_owned()),
            limitations: vec!["Synthetic fixture; not a scientific result".to_owned()],
        }
    }

    fn fixture() -> Dataset {
        Dataset {
            schema_version: SCHEMA_VERSION.to_owned(),
            sources: vec![SourceRecord {
                id: "fixture:source-1".to_owned(),
                title: "Synthetic test source (not a historical assertion)".to_owned(),
                source_uri: "urn:example:synthetic-source-1".to_owned(),
                rights: rights_open(),
            }],
            claims: vec![
                HistoricalClaim {
                    id: "fixture:claim-a".to_owned(),
                    proposition: "Synthetic fixture claim A".to_owned(),
                    mode: ClaimMode::Attested,
                    valid_time: Some(TimeInterval {
                        calendar: "fixture-year-scale".to_owned(),
                        start_year: 100,
                        end_year: 100,
                        precision: DatePrecision::ExactYear,
                    }),
                    evidence: vec![evidence("fixture:source-1")],
                    interpretation_note: None,
                },
                HistoricalClaim {
                    id: "fixture:claim-b".to_owned(),
                    proposition: "Synthetic fixture claim B".to_owned(),
                    mode: ClaimMode::Attested,
                    valid_time: None,
                    evidence: vec![evidence("fixture:source-1")],
                    interpretation_note: None,
                },
            ],
            relations: vec![KnowledgeRelation {
                id: "fixture:relation-1".to_owned(),
                from_claim_id: "fixture:claim-a".to_owned(),
                to_claim_id: "fixture:claim-b".to_owned(),
                kind: RelationKind::Supports,
                status: RelationStatus::Documented,
                evidence: vec![evidence("fixture:source-1")],
                interpretation_note: None,
            }],
            assets: vec![AssetRecord {
                id: "fixture:asset-1".to_owned(),
                source_id: "fixture:source-1".to_owned(),
                rights: RightsDeclaration {
                    status: RightsStatus::PublicDomain,
                    license_id: None,
                    attribution: None,
                    basis_note: Some("Synthetic fixture only".to_owned()),
                },
                distributable: true,
                content_sha256: "0".repeat(64),
            }],
            estimates: vec![],
            reconstruction_models: vec![],
            model_comparisons: vec![],
            estimate_contrasts: vec![],
            estimate_deltas: vec![],
            analysis_runs: vec![],
        }
    }

    #[test]
    fn accepts_well_formed_synthetic_fixture() {
        let report = validate(&fixture());
        assert!(report.is_valid(), "{:#?}", report.issues);
    }

    #[test]
    fn attested_claim_without_evidence_fails_closed() {
        let mut data = fixture();
        data.claims[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingEvidenceForClaim
        }));
    }

    #[test]
    fn challenging_only_evidence_does_not_support_an_attested_claim() {
        let mut data = fixture();
        data.claims[0].evidence[0].relation = EvidenceRelation::Challenges;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingEvidenceForClaim
        }));
    }

    #[test]
    fn challenging_evidence_is_preserved_alongside_support() {
        let mut data = fixture();
        let mut challenge = evidence("fixture:source-1");
        challenge.relation = EvidenceRelation::Challenges;
        challenge.locator = "fixture:paragraph-2".to_owned();
        data.claims[0].evidence.push(challenge);
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
        assert_eq!(data.claims[0].evidence.len(), 2);
        assert!(data.claims[0]
            .evidence
            .iter()
            .any(|link| link.relation == EvidenceRelation::Challenges));
    }

    #[test]
    fn documented_relation_with_challenging_only_evidence_fails() {
        let mut data = fixture();
        data.relations[0].evidence[0].relation = EvidenceRelation::Challenges;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::MissingEvidenceForRelation
        }));
    }

    #[test]
    fn dangling_source_reference_is_rejected() {
        let mut data = fixture();
        data.claims[0].evidence[0].source_id = "missing:source".to_owned();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::MissingSourceReference
                && item.message.contains("missing:source")
        }));
    }

    #[test]
    fn reversed_time_interval_is_rejected() {
        let mut data = fixture();
        data.claims[0].valid_time.as_mut().unwrap().start_year = 500;
        data.claims[0].valid_time.as_mut().unwrap().end_year = 400;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::InvalidTimeInterval
        }));
    }

    #[test]
    fn unknown_rights_cannot_be_distributed() {
        let mut data = fixture();
        data.assets[0].rights.status = RightsStatus::Unknown;
        data.assets[0].rights.license_id = None;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:asset-1"
                && item.code == ValidationCode::UndistributableRights
        }));
    }

    #[test]
    fn public_domain_asset_requires_recorded_basis() {
        let mut data = fixture();
        data.assets[0].rights.basis_note = None;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:asset-1"
                && item.code == ValidationCode::InvalidRightsDeclaration
        }));
    }

    #[test]
    fn open_license_requires_exact_identifier() {
        let mut data = fixture();
        data.assets[0].rights.license_id = None;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:asset-1"
                && item.code == ValidationCode::InvalidRightsDeclaration
        }));
    }

    #[test]
    fn documented_relation_requires_evidence() {
        let mut data = fixture();
        data.relations[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::MissingEvidenceForRelation
        }));
    }

    #[test]
    fn hypothesis_must_be_labelled_and_explained() {
        let mut data = fixture();
        data.claims[0].mode = ClaimMode::Hypothesis;
        data.claims[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingInterpretationNote
        }));
        assert!(!report.issues.iter().any(|item| {
            item.record_id == "fixture:claim-a"
                && item.code == ValidationCode::MissingEvidenceForClaim
        }));
    }

    #[test]
    fn duplicate_ids_are_rejected_across_record_kinds() {
        let mut data = fixture();
        data.assets[0].id = data.claims[0].id.clone();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::DuplicateId
                && item.record_id == "fixture:claim-a"
        }));
    }

    #[test]
    fn hypothesis_relation_requires_explanation() {
        let mut data = fixture();
        data.relations[0].status = RelationStatus::Hypothesized;
        data.relations[0].evidence.clear();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::MissingInterpretationNote
        }));
    }

    #[test]
    fn backwards_temporal_edge_is_rejected_when_intervals_decide_it() {
        let mut data = fixture();
        data.claims[0].valid_time.as_mut().unwrap().start_year = 500;
        data.claims[0].valid_time.as_mut().unwrap().end_year = 500;
        data.claims[1].valid_time = Some(TimeInterval {
            calendar: "fixture-year-scale".to_owned(),
            start_year: 300,
            end_year: 350,
            precision: DatePrecision::BroadRange,
        });
        data.relations[0].kind = RelationKind::Influenced;
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:relation-1"
                && item.code == ValidationCode::TemporalContradiction
        }));
    }

    #[test]
    fn overlapping_intervals_are_not_assumed_to_disprove_influence() {
        let mut data = fixture();
        data.claims[0].valid_time.as_mut().unwrap().start_year = 100;
        data.claims[0].valid_time.as_mut().unwrap().end_year = 120;
        data.claims[1].valid_time = Some(TimeInterval {
            calendar: "fixture-year-scale".to_owned(),
            start_year: 110,
            end_year: 140,
            precision: DatePrecision::BroadRange,
        });
        data.relations[0].kind = RelationKind::Influenced;
        assert!(
            validate(&data).issues.iter().all(|item| {
                item.code != ValidationCode::TemporalContradiction
            })
        );
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let mut data = fixture();
        data.schema_version = "99.0".to_owned();
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "dataset"
                && item.code == ValidationCode::UnsupportedSchemaVersion
        }));
    }

    #[test]
    fn antikythera_research_seed_has_valid_links_and_explicit_alternatives() {
        let raw = include_str!("../fixtures/antikythera-research-seed.json");
        let data: Dataset = serde_json::from_str(raw).expect("parse Antikythera research seed");
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
        assert_eq!(data.sources.len(), 7);
        assert_eq!(data.claims.len(), 9);
        assert_eq!(data.estimates.len(), 30);
        assert_eq!(data.analysis_runs.len(), 1);
        assert_eq!(data.analysis_runs[0].status, AnalysisRunStatus::Planned);
        assert!(data.analysis_runs[0].input_artifacts.is_empty());
        assert!(data.analysis_runs[0].output_artifacts.is_empty());
        assert_eq!(data.reconstruction_models.len(), 3);
        assert_eq!(data.model_comparisons.len(), 1);
        assert!(data.model_comparisons[0].model_ids.len() >= 2);
        assert_eq!(data.model_comparisons[0].assessments.len(), 4);
        assert!(data.model_comparisons[0].assessments.iter().any(|assessment| {
            assessment.status == AssessmentStatus::NotEvaluated
        }));
    }

    #[test]
    fn synthetic_json_fixture_deserializes_and_validates() {
        let raw = include_str!("../fixtures/synthetic-validation.json");
        let data: Dataset = serde_json::from_str(raw).expect("parse synthetic validation fixture");
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
        assert_eq!(data.analysis_runs.len(), 1);
        assert_eq!(data.analysis_runs[0].status, AnalysisRunStatus::Planned);
    }

    #[test]
    fn planned_analysis_run_does_not_claim_execution() {
        let mut data = fixture();
        data.analysis_runs.push(run_for_fixture(AnalysisRunStatus::Planned));
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
    }

    #[test]
    fn executed_analysis_run_requires_inputs_environment_and_output_digest() {
        let mut data = fixture();
        let mut run = run_for_fixture(AnalysisRunStatus::Executed);
        run.input_manifest_sha256 = None;
        run.environment_fingerprint = None;
        run.output_artifacts.clear();
        data.analysis_runs.push(run);
        let report = validate(&data);
        assert!(report.issues.iter().filter(|item| {
            item.record_id == "fixture:run-1"
                && item.code == ValidationCode::InvalidAnalysisRun
        }).count() >= 3);
    }

    #[test]
    fn reproduction_run_requires_existing_source_and_estimate_targets() {
        let mut data = fixture();
        data.estimates = vec![estimate_for_fixture("fixture:estimate-a", "fixture:claim-a")];
        let mut run = run_for_fixture(AnalysisRunStatus::Reproduced);
        run.comparison_source_id = Some("missing:source".to_owned());
        run.target_estimate_ids = vec!["missing:estimate".to_owned()];
        data.analysis_runs.push(run);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:run-1"
                && item.code == ValidationCode::InvalidAnalysisRun
                && item.message.contains("comparison_source_id")
        }));
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:run-1"
                && item.code == ValidationCode::MissingEstimateReference
        }));
    }

    #[test]
    fn reproduced_run_requires_preregistered_comparison_rule() {
        let mut data = fixture();
        data.estimates = vec![estimate_for_fixture("fixture:estimate-a", "fixture:claim-a")];
        let mut run = run_for_fixture(AnalysisRunStatus::Reproduced);
        run.comparison_rule = None;
        data.analysis_runs.push(run);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:run-1"
                && item.code == ValidationCode::EmptyField
                && item.message.contains("preregistered comparison rule")
        }));
    }

    #[test]
    fn valid_reproduced_run_is_linked_to_source_inputs_outputs_and_targets() {
        let mut data = fixture();
        data.estimates = vec![estimate_for_fixture("fixture:estimate-a", "fixture:claim-a")];
        data.analysis_runs.push(run_for_fixture(AnalysisRunStatus::Reproduced));
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
    }

    #[test]
    fn analysis_run_rejects_malformed_content_digests() {
        let mut data = fixture();
        let mut run = run_for_fixture(AnalysisRunStatus::InputsVerified);
        run.input_artifacts[0].sha256 = "not-a-digest".to_owned();
        data.analysis_runs.push(run);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:run-1"
                && item.code == ValidationCode::InvalidAnalysisRun
                && item.message.contains("SHA-256")
        }));
    }

    #[test]
    fn exact_decimal_arithmetic_preserves_source_precision() {
        assert_eq!(subtract_decimal_strings("0.3", "0.2").as_deref(), Some("0.1"));
        assert_eq!(
            subtract_decimal_strings("0.2714", "0.1917").as_deref(),
            Some("0.0797")
        );
        assert_eq!(
            subtract_decimal_strings("354.08", "355.24").as_deref(),
            Some("-1.16")
        );
        assert!(parse_decimal_parts("1e3").is_none());
        assert!(parse_decimal_parts("NaN").is_none());
    }

    #[test]
    fn complete_source_posterior_table_has_all_integer_counts_and_rounded_sums() {
        let raw = include_str!("../fixtures/antikythera-research-seed.json");
        let data: Dataset = serde_json::from_str(raw).expect("parse Antikythera research seed");
        let probabilities: Vec<_> = data
            .estimates
            .iter()
            .filter(|estimate| estimate.id.starts_with("antikythera:posterior-all-")
                || estimate.id.starts_with("antikythera:posterior-trim-"))
            .collect();
        assert_eq!(probabilities.len(), 26);

        for n in 349..=361 {
            assert!(probabilities.iter().any(|estimate| estimate.id == format!("antikythera:posterior-all-{n}")));
            assert!(probabilities.iter().any(|estimate| estimate.id == format!("antikythera:posterior-trim-{n}")));
            assert!(data.estimate_deltas.iter().any(|delta| delta.id == format!("antikythera:delta-posterior-n{n}-trimmed")));
        }

        let sum_for = |prefix: &str| -> f64 {
            probabilities.iter()
                .filter(|estimate| estimate.id.starts_with(prefix))
                .map(|estimate| estimate.value_decimal.parse::<f64>().expect("table value"))
                .sum()
        };
        // Source values are transcribed to four decimals. The sums therefore check rounding,
        // not posterior normalization at full numerical precision.
        assert!((sum_for("antikythera:posterior-all-") - 0.9998).abs() < 1e-9);
        assert!((sum_for("antikythera:posterior-trim-") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn antikythera_full_vs_trimmed_posterior_deltas_match_source_table() {
        let raw = include_str!("../fixtures/antikythera-research-seed.json");
        let data: Dataset = serde_json::from_str(raw).expect("parse Antikythera research seed");
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
        assert_eq!(data.estimate_deltas.len(), 13);

        let n354 = data.estimate_deltas.iter()
            .find(|delta| delta.id == "antikythera:delta-posterior-n354-trimmed")
            .unwrap();
        let result = analyze_estimate_delta(&data, n354).unwrap();
        assert_eq!(result.delta_value_decimal, "0.0797");
        assert_eq!(result.direction, DeltaDirection::Increase);

        let n355 = data.estimate_deltas.iter()
            .find(|delta| delta.id == "antikythera:delta-posterior-n355-trimmed")
            .unwrap();
        let result = analyze_estimate_delta(&data, n355).unwrap();
        assert_eq!(result.delta_value_decimal, "-0.0572");
        assert_eq!(result.direction, DeltaDirection::Decrease);

        let n356 = data.estimate_deltas.iter()
            .find(|delta| delta.id == "antikythera:delta-posterior-n356-trimmed")
            .unwrap();
        let result = analyze_estimate_delta(&data, n356).unwrap();
        assert_eq!(result.delta_value_decimal, "-0.1306");
        assert_eq!(result.direction, DeltaDirection::Decrease);
        assert!(result.dependence_note.contains("not independent"));
    }

    #[test]
    fn delta_rejects_mismatched_units_or_uncertainty_intervals() {
        let mut data = fixture();
        data.estimates = vec![
            estimate_for_fixture("fixture:estimate-a", "fixture:claim-a"),
            estimate_for_fixture("fixture:estimate-b", "fixture:claim-b"),
        ];
        data.estimates[0].interval = None;
        data.estimates[0].kind = EstimateKind::ReportedPoint;
        data.estimates[1].interval = None;
        data.estimates[1].kind = EstimateKind::ReportedPoint;
        data.estimates[1].unit = "seconds".to_owned();
        data.estimate_deltas.push(EstimateDelta {
            id: "fixture:delta".to_owned(),
            baseline_estimate_id: "fixture:estimate-a".to_owned(),
            perturbed_estimate_id: "fixture:estimate-b".to_owned(),
            variant_description: "Synthetic mismatched units".to_owned(),
            dependence_note: "Synthetic".to_owned(),
            interpretation_limit: "Not an inference".to_owned(),
        });
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:delta"
                && item.code == ValidationCode::InvalidEstimateDelta
                && item.message.contains("same quantity and unit")
        }));
        assert!(analyze_estimate_delta(&data, &data.estimate_deltas[0]).is_err());
    }

    #[test]
    fn antikythera_point_interval_diagnostics_are_descriptive_and_reproducible() {
        let raw = include_str!("../fixtures/antikythera-research-seed.json");
        let data: Dataset = serde_json::from_str(raw).expect("parse Antikythera research seed");
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
        assert_eq!(data.estimate_contrasts.len(), 4);

        let contrast_365 = data
            .estimate_contrasts
            .iter()
            .find(|c| c.id == "antikythera:contrast-365-vs-all-data-interval")
            .unwrap();
        let diagnostic = analyze_estimate_contrast(&data, contrast_365).unwrap();
        assert_eq!(diagnostic.position, IntervalPosition::Above);
        assert_eq!(diagnostic.distance_to_interval_decimal, "8.37");
        assert_eq!(diagnostic.coverage_percent.as_deref(), Some("68"));

        let contrast_354 = data
            .estimate_contrasts
            .iter()
            .find(|c| c.id == "antikythera:contrast-354-vs-all-data-interval")
            .unwrap();
        let diagnostic = analyze_estimate_contrast(&data, contrast_354).unwrap();
        assert_eq!(diagnostic.position, IntervalPosition::Within);
        assert_eq!(diagnostic.distance_to_interval_decimal, "0");
        assert!(diagnostic.interpretation_limit.contains("does not by itself establish"));
    }

    #[test]
    fn contrast_with_incompatible_units_fails_validation_and_analysis() {
        let mut data = fixture();
        data.estimates = vec![
            estimate_for_fixture("fixture:estimate-a", "fixture:claim-a"),
            estimate_for_fixture("fixture:estimate-b", "fixture:claim-b"),
        ];
        data.estimates[0].interval = None;
        data.estimates[0].kind = EstimateKind::ReportedPoint;
        data.estimates[1].unit = "seconds".to_owned();
        data.estimate_contrasts.push(EstimateContrast {
            id: "fixture:contrast".to_owned(),
            candidate_estimate_id: "fixture:estimate-a".to_owned(),
            reference_estimate_id: "fixture:estimate-b".to_owned(),
            purpose: "Synthetic unit mismatch negative control".to_owned(),
            dependence_note: "Synthetic".to_owned(),
            interpretation_limit: "Not an inference".to_owned(),
        });
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:contrast"
                && item.code == ValidationCode::InvalidEstimateContrast
                && item.message.contains("same quantity and unit")
        }));
        assert!(analyze_estimate_contrast(&data, &data.estimate_contrasts[0]).is_err());
    }

    #[test]
    fn contrast_requires_a_reference_interval() {
        let mut data = fixture();
        data.estimates = vec![
            estimate_for_fixture("fixture:estimate-a", "fixture:claim-a"),
            estimate_for_fixture("fixture:estimate-b", "fixture:claim-b"),
        ];
        data.estimates[0].interval = None;
        data.estimates[0].kind = EstimateKind::ReportedPoint;
        data.estimates[1].interval = None;
        data.estimates[1].kind = EstimateKind::ReportedPoint;
        data.estimate_contrasts.push(EstimateContrast {
            id: "fixture:contrast".to_owned(),
            candidate_estimate_id: "fixture:estimate-a".to_owned(),
            reference_estimate_id: "fixture:estimate-b".to_owned(),
            purpose: "Synthetic missing interval negative control".to_owned(),
            dependence_note: "Synthetic".to_owned(),
            interpretation_limit: "Not an inference".to_owned(),
        });
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:contrast"
                && item.code == ValidationCode::InvalidEstimateContrast
                && item.message.contains("reported uncertainty interval")
        }));
    }

    #[test]
    fn accepts_quantitative_estimates_and_explicit_model_comparison() {
        let mut data = fixture();
        data.estimates = vec![
            estimate_for_fixture("fixture:estimate-a", "fixture:claim-a"),
            estimate_for_fixture("fixture:estimate-b", "fixture:claim-b"),
        ];
        data.reconstruction_models = vec![
            model_for_fixture("fixture:model-a", "fixture:claim-a", "fixture:estimate-a"),
            model_for_fixture("fixture:model-b", "fixture:claim-b", "fixture:estimate-b"),
        ];
        data.model_comparisons = vec![ModelComparison {
            id: "fixture:comparison".to_owned(),
            question: "Which synthetic model better matches the test criterion?".to_owned(),
            model_ids: vec!["fixture:model-a".to_owned(), "fixture:model-b".to_owned()],
            evaluation_axes: vec!["residual".to_owned(), "assumption sensitivity".to_owned()],
            decision_rule: "Report axes separately; no aggregate winner".to_owned(),
            caveats: vec!["Synthetic fixture only".to_owned()],
            assessments: vec![],
        }];
        let report = validate(&data);
        assert!(report.is_valid(), "{:#?}", report.issues);
    }

    #[test]
    fn rejects_non_finite_numeric_estimate() {
        let mut data = fixture();
        let mut estimate = estimate_for_fixture("fixture:estimate-a", "fixture:claim-a");
        estimate.value_decimal = "NaN".to_owned();
        data.estimates.push(estimate);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:estimate-a"
                && item.code == ValidationCode::InvalidNumericEstimate
        }));
    }

    #[test]
    fn rejects_reversed_uncertainty_interval() {
        let mut data = fixture();
        let mut estimate = estimate_for_fixture("fixture:estimate-a", "fixture:claim-a");
        estimate.interval.as_mut().unwrap().lower_decimal = "2.0".to_owned();
        estimate.interval.as_mut().unwrap().upper_decimal = "1.0".to_owned();
        data.estimates.push(estimate);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:estimate-a"
                && item.code == ValidationCode::InvalidUncertaintyInterval
        }));
    }

    #[test]
    fn credible_interval_requires_coverage_percent() {
        let mut data = fixture();
        let mut estimate = estimate_for_fixture("fixture:estimate-a", "fixture:claim-a");
        estimate.interval.as_mut().unwrap().coverage_percent = None;
        data.estimates.push(estimate);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:estimate-a"
                && item.code == ValidationCode::InvalidUncertaintyInterval
                && item.message.contains("coverage_percent")
        }));
    }

    #[test]
    fn estimate_requires_supporting_source_evidence() {
        let mut data = fixture();
        let mut estimate = estimate_for_fixture("fixture:estimate-a", "fixture:claim-a");
        estimate.evidence[0].relation = EvidenceRelation::Challenges;
        data.estimates.push(estimate);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:estimate-a"
                && item.code == ValidationCode::MissingEstimateEvidence
        }));
    }

    #[test]
    fn independently_reproduced_assessment_requires_versioned_artifact() {
        let mut data = fixture();
        data.estimates = vec![
            estimate_for_fixture("fixture:estimate-a", "fixture:claim-a"),
            estimate_for_fixture("fixture:estimate-b", "fixture:claim-b"),
        ];
        data.reconstruction_models = vec![
            model_for_fixture("fixture:model-a", "fixture:claim-a", "fixture:estimate-a"),
            model_for_fixture("fixture:model-b", "fixture:claim-b", "fixture:estimate-b"),
        ];
        data.model_comparisons = vec![ModelComparison {
            id: "fixture:comparison".to_owned(),
            question: "Compare two synthetic models".to_owned(),
            model_ids: vec!["fixture:model-a".to_owned(), "fixture:model-b".to_owned()],
            evaluation_axes: vec!["residual".to_owned()],
            decision_rule: "Report per axis".to_owned(),
            caveats: vec![],
            assessments: vec![ModelAssessment {
                model_id: "fixture:model-a".to_owned(),
                axis: "residual".to_owned(),
                status: AssessmentStatus::IndependentlyReproduced,
                summary: "Synthetic reproduced result".to_owned(),
                estimate_ids: vec!["fixture:estimate-a".to_owned()],
                evidence: vec![evidence("fixture:source-1")],
                interpretation_note: None,
                validation_artifact: None,
            }],
        }];
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:comparison"
                && item.code == ValidationCode::InvalidModelAssessment
                && item.message.contains("versioned validation artifact")
        }));
    }

    #[test]
    fn assessment_must_use_declared_axis_and_model() {
        let mut data = fixture();
        data.estimates = vec![
            estimate_for_fixture("fixture:estimate-a", "fixture:claim-a"),
            estimate_for_fixture("fixture:estimate-b", "fixture:claim-b"),
        ];
        data.reconstruction_models = vec![
            model_for_fixture("fixture:model-a", "fixture:claim-a", "fixture:estimate-a"),
            model_for_fixture("fixture:model-b", "fixture:claim-b", "fixture:estimate-b"),
        ];
        data.model_comparisons = vec![ModelComparison {
            id: "fixture:comparison".to_owned(),
            question: "Compare two synthetic models".to_owned(),
            model_ids: vec!["fixture:model-a".to_owned(), "fixture:model-b".to_owned()],
            evaluation_axes: vec!["residual".to_owned()],
            decision_rule: "Report per axis".to_owned(),
            caveats: vec![],
            assessments: vec![ModelAssessment {
                model_id: "fixture:model-a".to_owned(),
                axis: "invented axis".to_owned(),
                status: AssessmentStatus::NotEvaluated,
                summary: "Not evaluated".to_owned(),
                estimate_ids: vec![],
                evidence: vec![],
                interpretation_note: None,
                validation_artifact: None,
            }],
        }];
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:comparison"
                && item.code == ValidationCode::InvalidModelAssessment
                && item.message.contains("undeclared evaluation axis")
        }));
    }

    #[test]
    fn model_comparison_requires_distinct_existing_models() {
        let mut data = fixture();
        data.model_comparisons.push(ModelComparison {
            id: "fixture:comparison".to_owned(),
            question: "Compare models".to_owned(),
            model_ids: vec!["missing:model".to_owned(), "missing:model".to_owned()],
            evaluation_axes: vec!["fit".to_owned()],
            decision_rule: "No aggregate score".to_owned(),
            caveats: vec![],
            assessments: vec![],
        });
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:comparison"
                && item.code == ValidationCode::InvalidModelComparison
        }));
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:comparison"
                && item.code == ValidationCode::MissingModelReference
        }));
    }

    #[test]
    fn model_cannot_silently_reference_unknown_estimate() {
        let mut data = fixture();
        let mut model = model_for_fixture(
            "fixture:model-a",
            "fixture:claim-a",
            "missing:estimate",
        );
        model.estimate_ids = vec!["missing:estimate".to_owned()];
        data.reconstruction_models.push(model);
        let report = validate(&data);
        assert!(report.issues.iter().any(|item| {
            item.record_id == "fixture:model-a"
                && item.code == ValidationCode::MissingEstimateReference
        }));
    }

    #[test]
    fn report_order_is_stable() {
        let mut data_a = fixture();
        data_a.claims[0].evidence.clear();

        let mut data_b = data_a.clone();
        data_b.claims.reverse();

        assert_eq!(validate(&data_a).issues, validate(&data_b).issues);
    }

    #[test]
    fn json_round_trip_preserves_records_and_uncertainty() {
        let data = fixture();
        let encoded = serde_json::to_string(&data).expect("serialize fixture");
        let decoded: Dataset = serde_json::from_str(&encoded).expect("deserialize fixture");
        assert_eq!(decoded, data);
        assert_eq!(
            decoded.claims[0].valid_time.as_ref().unwrap().start_year,
            100
        );
    }
}
