// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Machine-checkable benchmark case and result contracts.
//!
//! This module validates the metadata around a scientific comparison. It does
//! not implement a physical model, verify source files against their digest,
//! or establish that a chosen reference is scientifically appropriate.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CASE_SCHEMA_V1: &str = "symtropy.natural_hazard_case.v1";
pub const RESULT_SCHEMA_V1: &str = "symtropy.natural_hazard_result.v1";

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Maximum evidence tier a case/report is permitted to claim.
/// This is a claim ceiling, not a quality score shared across hazards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimTier {
    Execution,
    StructuralInvariant,
    AnalyticalOrManufactured,
    PublishedReference,
    HeldOutObservation,
    DecisionImpact,
    Operational,
}

/// The rights state for a source used in the benchmark case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePermission {
    PublicOpen,
    PermissionGranted,
    RestrictedAuthorized,
    NotApplicable,
    Unknown,
    Prohibited,
}

impl SourcePermission {
    fn allows_use(self) -> bool {
        matches!(
            self,
            Self::PublicOpen
                | Self::PermissionGranted
                | Self::RestrictedAuthorized
                | Self::NotApplicable
        )
    }
}

/// Purpose for which a sample/event belongs to a case split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitRole {
    Calibration,
    Development,
    Holdout,
}

/// One source record used by the case, with permission and content identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkSource {
    pub source_id: String,
    pub publisher: String,
    pub dataset_version: String,
    pub permission: SourcePermission,
    /// Digest of the exact source artifact or normalized input bundle.
    pub digest_sha256: String,
    pub limitations: Vec<String>,
}

/// Scientific target and the support over which it is observed/scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkTarget {
    pub quantity: String,
    pub units: String,
    pub spatial_support: String,
    pub coordinate_reference: Option<String>,
    /// Valid interval on a declared UTC Unix-second axis.
    pub valid_start_unix_s: i64,
    pub valid_end_unix_s: i64,
    pub observation_operator: String,
}

/// Named model/dataset used as an independent reference or comparison baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonModel {
    pub model_id: String,
    pub version: String,
    pub comparability_statement: String,
    pub uncertainty_statement: String,
}

/// Predeclared metric and its measurement semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkMetric {
    pub metric_id: String,
    pub units: String,
    pub baseline_required: bool,
    /// Optional absolute tolerance; must be finite and non-negative.
    pub tolerance: Option<f64>,
    /// Optional decision/alert threshold fixed before evaluation.
    pub threshold: Option<f64>,
}

/// Explicit assignment of a sample and its independent event to one split.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitAssignment {
    pub sample_id: String,
    pub event_id: String,
    pub region_id: String,
    pub role: SplitRole,
}

/// Versioned contract for one scientific benchmark case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkCase {
    pub schema: String,
    pub case_id: String,
    pub claim_ceiling: ClaimTier,
    pub target: BenchmarkTarget,
    pub sources: Vec<BenchmarkSource>,
    pub reference: Option<ComparisonModel>,
    pub baseline: Option<ComparisonModel>,
    pub metrics: Vec<BenchmarkMetric>,
    /// Evidence records supporting this case's declared claim ceiling.
    pub evidence: Vec<super::EvidenceRecord>,
    pub splits: Vec<SplitAssignment>,
    pub input_digest_sha256: String,
    pub config_digest_sha256: String,
    /// Required only for claims involving decision utility or operational use.
    pub decision_utility_spec: Option<String>,
    pub operational_context: Option<String>,
    pub limitations: Vec<String>,
}

/// Errors that prevent a case contract from being used for a scored claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseValidationError {
    WrongSchema,
    EmptyIdentity,
    InvalidTarget,
    InvalidCaseDigest,
    NoSources,
    DuplicateSourceId(String),
    InvalidSource(String),
    SourcePermissionUnknown(String),
    SourceUseProhibited(String),
    InvalidComparisonModel(&'static str),
    MissingReferenceForClaim,
    MissingBaselineForClaim,
    MissingEvidence(super::EvidenceClass),
    FailedEvidence(super::EvidenceClass),
    UnavailableEvidence(super::EvidenceClass),
    NotApplicableEvidence(super::EvidenceClass),
    EmptyEvidenceSuite,
    NoMetrics,
    InvalidMetric(String),
    InvalidSplit(String),
    DuplicateSampleId(String),
    HoldoutLeakage(String),
    MissingCalibrationOrHoldout,
    MissingDecisionUtilitySpec,
    MissingOperationalContext,
}

impl BenchmarkCase {
    /// Validate case metadata and claim ceilings. This does not validate physics.
    pub fn validate(&self) -> Result<(), CaseValidationError> {
        if self.schema != CASE_SCHEMA_V1 {
            return Err(CaseValidationError::WrongSchema);
        }
        if !nonblank(&self.case_id) {
            return Err(CaseValidationError::EmptyIdentity);
        }
        if !nonblank(&self.target.quantity)
            || !nonblank(&self.target.units)
            || !nonblank(&self.target.spatial_support)
            || !nonblank(&self.target.observation_operator)
            || self.target.valid_start_unix_s >= self.target.valid_end_unix_s
            || self
                .target
                .coordinate_reference
                .as_deref()
                .is_some_and(|crs| !nonblank(crs))
        {
            return Err(CaseValidationError::InvalidTarget);
        }
        if !is_sha256(&self.input_digest_sha256) || !is_sha256(&self.config_digest_sha256) {
            return Err(CaseValidationError::InvalidCaseDigest);
        }
        if self.sources.is_empty() {
            return Err(CaseValidationError::NoSources);
        }

        let mut source_ids = BTreeSet::new();
        for source in &self.sources {
            if !nonblank(&source.source_id)
                || !nonblank(&source.publisher)
                || !nonblank(&source.dataset_version)
                || !is_sha256(&source.digest_sha256)
            {
                return Err(CaseValidationError::InvalidSource(source.source_id.clone()));
            }
            let source_key = key(&source.source_id);
            if !source_ids.insert(source_key) {
                return Err(CaseValidationError::DuplicateSourceId(
                    source.source_id.clone(),
                ));
            }
            if source.permission == SourcePermission::Unknown {
                return Err(CaseValidationError::SourcePermissionUnknown(
                    source.source_id.clone(),
                ));
            }
            if !source.permission.allows_use() {
                return Err(CaseValidationError::SourceUseProhibited(
                    source.source_id.clone(),
                ));
            }
        }

        validate_case_evidence(self.claim_ceiling, &self.evidence)?;
        if self.claim_ceiling >= ClaimTier::PublishedReference {
            validate_comparison(self.reference.as_ref(), "reference")?;
        }
        if self.claim_ceiling >= ClaimTier::HeldOutObservation {
            validate_comparison(self.baseline.as_ref(), "baseline")?;
        }
        if self.metrics.is_empty() {
            return Err(CaseValidationError::NoMetrics);
        }
        for metric in &self.metrics {
            if !nonblank(&metric.metric_id)
                || !nonblank(&metric.units)
                || metric
                    .tolerance
                    .is_some_and(|value| !value.is_finite() || value < 0.0)
                || metric.threshold.is_some_and(|value| !value.is_finite())
            {
                return Err(CaseValidationError::InvalidMetric(metric.metric_id.clone()));
            }
            if metric.baseline_required && self.baseline.is_none() {
                return Err(CaseValidationError::MissingBaselineForClaim);
            }
        }

        if self.claim_ceiling >= ClaimTier::HeldOutObservation {
            validate_splits(&self.splits)?;
        } else {
            // Validate any declared splits even for low-tier cases.
            validate_split_identities(&self.splits)?;
        }
        if self.claim_ceiling >= ClaimTier::DecisionImpact
            && self
                .decision_utility_spec
                .as_deref()
                .is_none_or(|value| !nonblank(value))
        {
            return Err(CaseValidationError::MissingDecisionUtilitySpec);
        }
        if self.claim_ceiling >= ClaimTier::Operational
            && self
                .operational_context
                .as_deref()
                .is_none_or(|value| !nonblank(value))
        {
            return Err(CaseValidationError::MissingOperationalContext);
        }
        Ok(())
    }
}

fn validate_case_evidence(
    claim_ceiling: ClaimTier,
    records: &[super::EvidenceRecord],
) -> Result<(), CaseValidationError> {
    let mut suite_keys = BTreeSet::new();
    for record in records {
        if !nonblank(&record.suite_id) || !nonblank(&record.detail) {
            return Err(CaseValidationError::EmptyEvidenceSuite);
        }
        if record
            .source_uri
            .as_deref()
            .is_some_and(|uri| !uri.starts_with("https://"))
        {
            return Err(CaseValidationError::EmptyEvidenceSuite);
        }
        if !suite_keys.insert((record.class, key(&record.suite_id))) {
            return Err(CaseValidationError::EmptyEvidenceSuite);
        }
    }

    // Each tier requires its own evidence; presentation quality is never a
    // substitute for physical, observational, impact, or operational evidence.
    let requirements: Vec<Vec<super::EvidenceClass>> = match claim_ceiling {
        ClaimTier::Execution => vec![],
        ClaimTier::StructuralInvariant => vec![vec![super::EvidenceClass::StructuralInvariant]],
        ClaimTier::AnalyticalOrManufactured => vec![
            vec![super::EvidenceClass::StructuralInvariant],
            vec![super::EvidenceClass::AnalyticalOrManufactured],
        ],
        ClaimTier::PublishedReference => vec![
            vec![super::EvidenceClass::StructuralInvariant],
            vec![super::EvidenceClass::AnalyticalOrManufactured],
            vec![
                super::EvidenceClass::PublishedBenchmark,
                super::EvidenceClass::ReferenceDifferential,
            ],
        ],
        ClaimTier::HeldOutObservation => vec![
            vec![super::EvidenceClass::StructuralInvariant],
            vec![super::EvidenceClass::AnalyticalOrManufactured],
            vec![
                super::EvidenceClass::PublishedBenchmark,
                super::EvidenceClass::ReferenceDifferential,
            ],
            vec![super::EvidenceClass::HeldOutObservation],
        ],
        ClaimTier::DecisionImpact => vec![
            vec![super::EvidenceClass::StructuralInvariant],
            vec![super::EvidenceClass::AnalyticalOrManufactured],
            vec![
                super::EvidenceClass::PublishedBenchmark,
                super::EvidenceClass::ReferenceDifferential,
            ],
            vec![super::EvidenceClass::HeldOutObservation],
            vec![super::EvidenceClass::DecisionImpact],
        ],
        ClaimTier::Operational => vec![
            vec![super::EvidenceClass::StructuralInvariant],
            vec![super::EvidenceClass::AnalyticalOrManufactured],
            vec![
                super::EvidenceClass::PublishedBenchmark,
                super::EvidenceClass::ReferenceDifferential,
            ],
            vec![super::EvidenceClass::HeldOutObservation],
            vec![super::EvidenceClass::DecisionImpact],
            vec![super::EvidenceClass::OperationalSkill],
        ],
    };

    for accepted_classes in requirements {
        let matching: Vec<_> = records
            .iter()
            .filter(|record| accepted_classes.contains(&record.class))
            .collect();
        if matching.is_empty() {
            return Err(CaseValidationError::MissingEvidence(accepted_classes[0]));
        }
        for record in &matching {
            match record.status {
                super::EvidenceStatus::Pass => {}
                super::EvidenceStatus::Fail => {
                    return Err(CaseValidationError::FailedEvidence(record.class));
                }
                super::EvidenceStatus::Unavailable => {
                    return Err(CaseValidationError::UnavailableEvidence(record.class));
                }
                super::EvidenceStatus::NotApplicable => {
                    return Err(CaseValidationError::NotApplicableEvidence(record.class));
                }
            }
        }
    }
    Ok(())
}

fn validate_comparison(
    comparison: Option<&ComparisonModel>,
    kind: &'static str,
) -> Result<(), CaseValidationError> {
    let Some(comparison) = comparison else {
        return Err(if kind == "reference" {
            CaseValidationError::MissingReferenceForClaim
        } else {
            CaseValidationError::MissingBaselineForClaim
        });
    };
    if !nonblank(&comparison.model_id)
        || !nonblank(&comparison.version)
        || !nonblank(&comparison.comparability_statement)
        || !nonblank(&comparison.uncertainty_statement)
    {
        return Err(CaseValidationError::InvalidComparisonModel(kind));
    }
    Ok(())
}

fn validate_split_identities(splits: &[SplitAssignment]) -> Result<(), CaseValidationError> {
    let mut samples = BTreeSet::new();
    for split in splits {
        if !nonblank(&split.sample_id) || !nonblank(&split.event_id) || !nonblank(&split.region_id)
        {
            return Err(CaseValidationError::InvalidSplit(split.sample_id.clone()));
        }
        if !samples.insert(key(&split.sample_id)) {
            return Err(CaseValidationError::DuplicateSampleId(
                split.sample_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_splits(splits: &[SplitAssignment]) -> Result<(), CaseValidationError> {
    validate_split_identities(splits)?;
    let mut event_roles: BTreeMap<String, BTreeSet<SplitRole>> = BTreeMap::new();
    let mut roles = BTreeSet::new();
    for split in splits {
        roles.insert(split.role);
        event_roles
            .entry(key(&split.event_id))
            .or_default()
            .insert(split.role);
    }
    for (event_id, event_role_set) in event_roles {
        if event_role_set.contains(&SplitRole::Holdout)
            && event_role_set
                .iter()
                .any(|role| *role != SplitRole::Holdout)
        {
            return Err(CaseValidationError::HoldoutLeakage(event_id));
        }
    }
    if !roles.contains(&SplitRole::Calibration) || !roles.contains(&SplitRole::Holdout) {
        return Err(CaseValidationError::MissingCalibrationOrHoldout);
    }
    Ok(())
}

/// Outcome for one frozen case/model execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkRunStatus {
    Pass,
    Fail,
    Unavailable,
    OutOfDomain,
    Excluded,
}

/// Digest-bound result record. It records evidence; it does not compute the score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub schema: String,
    pub result_id: String,
    pub case_id: String,
    pub model_revision: String,
    pub claim_tier: ClaimTier,
    pub status: BenchmarkRunStatus,
    pub input_digest_sha256: String,
    pub config_digest_sha256: String,
    pub output_digest_sha256: Option<String>,
    pub evidence_artifact_digests_sha256: Vec<String>,
    pub scored_sample_count: u64,
    pub excluded_sample_count: u64,
    pub status_reason: Option<String>,
}

/// Errors that prevent a result from being attached to its case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultValidationError {
    InvalidResultSchema,
    EmptyIdentity,
    CaseInvalid(CaseValidationError),
    WrongCase,
    ClaimExceedsCaseCeiling,
    DigestInvalid,
    CaseDigestMismatch,
    PassingResultMissingArtifact,
    PassingResultHasNoScoredSamples,
    MissingReasonForNonPass,
}

impl BenchmarkResult {
    /// Validate the result against its declared case and claim ceiling.
    pub fn validate_against(&self, case: &BenchmarkCase) -> Result<(), ResultValidationError> {
        case.validate()
            .map_err(ResultValidationError::CaseInvalid)?;
        if self.schema != RESULT_SCHEMA_V1 {
            return Err(ResultValidationError::InvalidResultSchema);
        }
        if !nonblank(&self.result_id) || !nonblank(&self.model_revision) {
            return Err(ResultValidationError::EmptyIdentity);
        }
        if key(&self.case_id) != key(&case.case_id) {
            return Err(ResultValidationError::WrongCase);
        }
        if self.claim_tier > case.claim_ceiling {
            return Err(ResultValidationError::ClaimExceedsCaseCeiling);
        }
        if !is_sha256(&self.input_digest_sha256)
            || !is_sha256(&self.config_digest_sha256)
            || self
                .output_digest_sha256
                .as_deref()
                .is_some_and(|digest| !is_sha256(digest))
            || self
                .evidence_artifact_digests_sha256
                .iter()
                .any(|digest| !is_sha256(digest))
        {
            return Err(ResultValidationError::DigestInvalid);
        }
        if self.input_digest_sha256 != case.input_digest_sha256
            || self.config_digest_sha256 != case.config_digest_sha256
        {
            return Err(ResultValidationError::CaseDigestMismatch);
        }
        match self.status {
            BenchmarkRunStatus::Pass => {
                if self.output_digest_sha256.is_none()
                    || self.evidence_artifact_digests_sha256.is_empty()
                {
                    return Err(ResultValidationError::PassingResultMissingArtifact);
                }
                if self.scored_sample_count == 0 {
                    return Err(ResultValidationError::PassingResultHasNoScoredSamples);
                }
            }
            BenchmarkRunStatus::Fail
            | BenchmarkRunStatus::Unavailable
            | BenchmarkRunStatus::OutOfDomain
            | BenchmarkRunStatus::Excluded => {
                if self
                    .status_reason
                    .as_deref()
                    .is_none_or(|reason| !nonblank(reason))
                {
                    return Err(ResultValidationError::MissingReasonForNonPass);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comparison(name: &str) -> ComparisonModel {
        ComparisonModel {
            model_id: name.into(),
            version: "v1".into(),
            comparability_statement: "Same target, units, support, mask and valid times".into(),
            uncertainty_statement: "Reference uncertainty is recorded with the case".into(),
        }
    }

    fn case() -> BenchmarkCase {
        BenchmarkCase {
            schema: CASE_SCHEMA_V1.into(),
            case_id: "flood.synthetic.lake-at-rest.v1".into(),
            claim_ceiling: ClaimTier::AnalyticalOrManufactured,
            target: BenchmarkTarget {
                quantity: "water_depth".into(),
                units: "m".into(),
                spatial_support: "cell-centred grid over the declared closed basin".into(),
                coordinate_reference: Some("EPSG:4326".into()),
                valid_start_unix_s: 1_800_000_000,
                valid_end_unix_s: 1_800_003_600,
                observation_operator: "cell mean at model-valid time".into(),
            },
            sources: vec![BenchmarkSource {
                source_id: "synthetic-initial-condition".into(),
                publisher: "test-fixture".into(),
                dataset_version: "v1".into(),
                permission: SourcePermission::NotApplicable,
                digest_sha256: "a".repeat(64),
                limitations: vec!["synthetic only; not observational evidence".into()],
            }],
            reference: Some(comparison("analytic-lake-at-rest")),
            baseline: Some(comparison("static-baseline")),
            metrics: vec![BenchmarkMetric {
                metric_id: "max_depth_error".into(),
                units: "m".into(),
                baseline_required: true,
                tolerance: Some(1.0e-6),
                threshold: None,
            }],
            evidence: vec![
                super::EvidenceRecord {
                    class: super::EvidenceClass::StructuralInvariant,
                    status: super::EvidenceStatus::Pass,
                    suite_id: "closed-basin-budget-v1".into(),
                    source_uri: None,
                    detail: "Synthetic contract fixture for structural checks".into(),
                },
                super::EvidenceRecord {
                    class: super::EvidenceClass::AnalyticalOrManufactured,
                    status: super::EvidenceStatus::Pass,
                    suite_id: "lake-at-rest-analytic-v1".into(),
                    source_uri: None,
                    detail: "Synthetic analytical target used only to test schema behavior".into(),
                },
            ],
            splits: vec![
                SplitAssignment {
                    sample_id: "cal-sample-1".into(),
                    event_id: "cal-event-1".into(),
                    region_id: "region-a".into(),
                    role: SplitRole::Calibration,
                },
                SplitAssignment {
                    sample_id: "holdout-sample-1".into(),
                    event_id: "heldout-event-1".into(),
                    region_id: "region-b".into(),
                    role: SplitRole::Holdout,
                },
            ],
            input_digest_sha256: "b".repeat(64),
            config_digest_sha256: "c".repeat(64),
            decision_utility_spec: None,
            operational_context: None,
            limitations: vec!["Synthetic fixture qualifies contract behavior only".into()],
        }
    }

    fn heldout_case() -> BenchmarkCase {
        let mut spec = case();
        spec.claim_ceiling = ClaimTier::HeldOutObservation;
        spec.evidence.push(super::EvidenceRecord {
            class: super::EvidenceClass::PublishedBenchmark,
            status: super::EvidenceStatus::Pass,
            suite_id: "published-lake-at-rest-v1".into(),
            source_uri: Some("https://example.org/benchmark".into()),
            detail: "Synthetic metadata fixture for reference-tier validation".into(),
        });
        spec.evidence.push(super::EvidenceRecord {
            class: super::EvidenceClass::HeldOutObservation,
            status: super::EvidenceStatus::Pass,
            suite_id: "heldout-observation-v1".into(),
            source_uri: Some("https://example.org/heldout".into()),
            detail: "Synthetic metadata fixture for evidence-tier validation".into(),
        });
        spec
    }

    fn passing_result(case: &BenchmarkCase) -> BenchmarkResult {
        BenchmarkResult {
            schema: RESULT_SCHEMA_V1.into(),
            result_id: "run-001".into(),
            case_id: case.case_id.clone(),
            model_revision: "0123456789abcdef".into(),
            claim_tier: case.claim_ceiling,
            status: BenchmarkRunStatus::Pass,
            input_digest_sha256: case.input_digest_sha256.clone(),
            config_digest_sha256: case.config_digest_sha256.clone(),
            output_digest_sha256: Some("d".repeat(64)),
            evidence_artifact_digests_sha256: vec!["e".repeat(64)],
            scored_sample_count: 1,
            excluded_sample_count: 0,
            status_reason: None,
        }
    }

    #[test]
    fn well_formed_case_has_explicit_target_rights_reference_baseline_and_holdout() {
        assert_eq!(case().validate(), Ok(()));
    }

    #[test]
    fn unknown_or_prohibited_source_rights_block_case_validation() {
        let mut unknown = case();
        unknown.sources[0].permission = SourcePermission::Unknown;
        assert_eq!(
            unknown.validate(),
            Err(CaseValidationError::SourcePermissionUnknown(
                "synthetic-initial-condition".into()
            ))
        );
        let mut prohibited = case();
        prohibited.sources[0].permission = SourcePermission::Prohibited;
        assert_eq!(
            prohibited.validate(),
            Err(CaseValidationError::SourceUseProhibited(
                "synthetic-initial-condition".into()
            ))
        );
    }

    #[test]
    fn reference_and_baseline_are_required_for_the_claimed_tier() {
        let mut no_reference = heldout_case();
        no_reference.reference = None;
        assert_eq!(
            no_reference.validate(),
            Err(CaseValidationError::MissingReferenceForClaim)
        );
        let mut no_baseline = heldout_case();
        no_baseline.baseline = None;
        assert_eq!(
            no_baseline.validate(),
            Err(CaseValidationError::MissingBaselineForClaim)
        );
    }

    #[test]
    fn normalized_event_identity_cannot_leak_from_calibration_to_holdout() {
        let mut leaked = heldout_case();
        leaked.splits[1].event_id = " CAL-EVENT-1 ".into();
        assert_eq!(
            leaked.validate(),
            Err(CaseValidationError::HoldoutLeakage("cal-event-1".into()))
        );
    }

    #[test]
    fn sample_identity_must_be_unique_across_roles() {
        let mut duplicate = heldout_case();
        duplicate.splits[1].sample_id = " CAL-SAMPLE-1 ".into();
        assert_eq!(
            duplicate.validate(),
            Err(CaseValidationError::DuplicateSampleId(
                " CAL-SAMPLE-1 ".into()
            ))
        );
    }

    #[test]
    fn bad_tolerances_and_malformed_digests_are_rejected() {
        let mut invalid_metric = case();
        invalid_metric.metrics[0].tolerance = Some(f64::NAN);
        assert_eq!(
            invalid_metric.validate(),
            Err(CaseValidationError::InvalidMetric("max_depth_error".into()))
        );
        let mut invalid_digest = case();
        invalid_digest.input_digest_sha256 = "not-a-digest".into();
        assert_eq!(
            invalid_digest.validate(),
            Err(CaseValidationError::InvalidCaseDigest)
        );
    }

    #[test]
    fn impact_and_operational_claims_require_their_own_context() {
        let mut impact = heldout_case();
        impact.claim_ceiling = ClaimTier::DecisionImpact;
        impact.evidence.push(super::EvidenceRecord {
            class: super::EvidenceClass::DecisionImpact,
            status: super::EvidenceStatus::Pass,
            suite_id: "cost-loss-evaluation-v1".into(),
            source_uri: None,
            detail: "Synthetic contract fixture for decision impact evidence".into(),
        });
        assert_eq!(
            impact.validate(),
            Err(CaseValidationError::MissingDecisionUtilitySpec)
        );
        impact.decision_utility_spec = Some("predeclared cost-loss table v1".into());
        assert_eq!(impact.validate(), Ok(()));
        impact.claim_ceiling = ClaimTier::Operational;
        impact.evidence.push(super::EvidenceRecord {
            class: super::EvidenceClass::OperationalSkill,
            status: super::EvidenceStatus::Pass,
            suite_id: "operational-verification-v1".into(),
            source_uri: None,
            detail: "Synthetic contract fixture for operational claim gating".into(),
        });
        assert_eq!(
            impact.validate(),
            Err(CaseValidationError::MissingOperationalContext)
        );
    }

    #[test]
    fn result_cannot_claim_beyond_case_ceiling_or_swap_case_digests() {
        let mut spec = case();
        spec.claim_ceiling = ClaimTier::StructuralInvariant;
        let mut result = passing_result(&spec);
        result.claim_tier = ClaimTier::HeldOutObservation;
        assert_eq!(
            result.validate_against(&spec),
            Err(ResultValidationError::ClaimExceedsCaseCeiling)
        );
        result.claim_tier = ClaimTier::StructuralInvariant;
        spec.input_digest_sha256 = "f".repeat(64);
        assert_eq!(
            result.validate_against(&spec),
            Err(ResultValidationError::CaseDigestMismatch)
        );
    }

    #[test]
    fn pass_requires_output_artifact_and_scored_samples() {
        let spec = case();
        let mut no_artifact = passing_result(&spec);
        no_artifact.evidence_artifact_digests_sha256.clear();
        assert_eq!(
            no_artifact.validate_against(&spec),
            Err(ResultValidationError::PassingResultMissingArtifact)
        );
        let mut no_samples = passing_result(&spec);
        no_samples.scored_sample_count = 0;
        assert_eq!(
            no_samples.validate_against(&spec),
            Err(ResultValidationError::PassingResultHasNoScoredSamples)
        );
    }

    #[test]
    fn low_tier_evidence_cannot_be_promoted_to_heldout_skill() {
        let mut spec = case();
        spec.claim_ceiling = ClaimTier::HeldOutObservation;
        assert_eq!(
            spec.validate(),
            Err(CaseValidationError::MissingEvidence(
                super::EvidenceClass::PublishedBenchmark
            ))
        );
    }

    #[test]
    fn nonpass_states_need_explanations_and_unknown_is_not_zero() {
        let spec = case();
        let mut unavailable = passing_result(&spec);
        unavailable.status = BenchmarkRunStatus::Unavailable;
        unavailable.status_reason = None;
        assert_eq!(
            unavailable.validate_against(&spec),
            Err(ResultValidationError::MissingReasonForNonPass)
        );
        unavailable.status_reason = Some("observation source unavailable".into());
        assert_eq!(unavailable.validate_against(&spec), Ok(()));
    }

    #[test]
    fn decision_impact_claim_requires_explicit_decision_evidence() {
        let mut spec = heldout_case();
        spec.claim_ceiling = ClaimTier::DecisionImpact;
        spec.decision_utility_spec = Some("pre-registered cost-loss policy".into());
        spec.evidence.push(super::EvidenceRecord {
            class: super::EvidenceClass::DecisionImpact,
            status: super::EvidenceStatus::Pass,
            suite_id: "impact-decision-value-v1".into(),
            source_uri: None,
            detail: "Synthetic metadata fixture for decision-value tier".into(),
        });
        assert_eq!(spec.validate(), Ok(()));
        let result = passing_result(&spec);
        assert_eq!(result.validate_against(&spec), Ok(()));
    }
}
