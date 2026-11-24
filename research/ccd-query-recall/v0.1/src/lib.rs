// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Query-level CCD recall evaluation, deliberately separate from broadphase
//! collider-pair accounting.
//!
//! Query IDs are independent of broadphase box IDs. The evaluator never joins
//! records by array position.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use nyu_ccd_query_import::{CcdQuery, QueryKind};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum QueryClass {
    EdgeEdge,
    VertexFace,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QueryIdentity {
    dataset: String,
    scene: String,
    time_step: u64,
    query_class: QueryClass,
    query_index: usize,
}

impl QueryIdentity {
    pub fn new(
        dataset: impl Into<String>,
        scene: impl Into<String>,
        time_step: u64,
        query_class: QueryClass,
        query_index: usize,
    ) -> Self {
        Self {
            dataset: dataset.into(),
            scene: scene.into(),
            time_step,
            query_class,
            query_index,
        }
    }

    pub fn from_imported(
        dataset: impl Into<String>,
        scene: impl Into<String>,
        time_step: u64,
        query: &CcdQuery,
    ) -> Self {
        let query_class = match query.kind() {
            QueryKind::EdgeEdge => QueryClass::EdgeEdge,
            QueryKind::VertexFace => QueryClass::VertexFace,
        };
        Self::new(dataset, scene, time_step, query_class, query.index())
    }

    #[must_use]
    pub fn dataset(&self) -> &str {
        &self.dataset
    }

    #[must_use]
    pub fn scene(&self) -> &str {
        &self.scene
    }

    #[must_use]
    pub fn time_step(&self) -> u64 {
        self.time_step
    }

    #[must_use]
    pub fn query_class(&self) -> QueryClass {
        self.query_class
    }

    #[must_use]
    pub fn query_index(&self) -> usize {
        self.query_index
    }

    fn valid(&self) -> bool {
        !self.dataset.trim().is_empty() && !self.scene.trim().is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackReason {
    UnsupportedMotion,
    UnknownNumericProfile,
    InvalidInput,
    ArithmeticOverflow,
    RejectedCertificate,
    MissingProof,
    CapacityLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    AcceptedAndReplayed,
    Rejected,
    NotRun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PruneEvidence {
    certificate_sha256: String,
    frozen_input_sha256: String,
    verifier_id: String,
    status: VerificationStatus,
}

impl PruneEvidence {
    pub fn new(
        certificate_sha256: impl Into<String>,
        frozen_input_sha256: impl Into<String>,
        verifier_id: impl Into<String>,
        status: VerificationStatus,
    ) -> Self {
        Self {
            certificate_sha256: certificate_sha256.into(),
            frozen_input_sha256: frozen_input_sha256.into(),
            verifier_id: verifier_id.into(),
            status,
        }
    }

    #[must_use]
    pub fn status(&self) -> VerificationStatus {
        self.status
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateDisposition {
    Forwarded,
    Fallback(FallbackReason),
    CertifiedPruned(PruneEvidence),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NarrowphaseOutcome {
    Collision,
    NoCollision,
    Indeterminate,
    NotRun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryEvaluation {
    identity: QueryIdentity,
    oracle_collision: bool,
    candidate: CandidateDisposition,
    narrowphase: NarrowphaseOutcome,
    narrowphase_elapsed_ns: Option<u64>,
}

impl QueryEvaluation {
    pub fn new(
        identity: QueryIdentity,
        oracle_collision: bool,
        candidate: CandidateDisposition,
        narrowphase: NarrowphaseOutcome,
    ) -> Self {
        Self {
            identity,
            oracle_collision,
            candidate,
            narrowphase,
            narrowphase_elapsed_ns: None,
        }
    }

    #[must_use]
    pub fn with_narrowphase_elapsed_ns(mut self, elapsed_ns: u64) -> Self {
        self.narrowphase_elapsed_ns = Some(elapsed_ns);
        self
    }

    /// Construct a result from an imported query after its independent Boolean
    /// ground-truth vector has been attached.
    pub fn from_imported(
        dataset: impl Into<String>,
        scene: impl Into<String>,
        time_step: u64,
        query: &CcdQuery,
        candidate: CandidateDisposition,
        narrowphase: NarrowphaseOutcome,
    ) -> Result<Self, EvaluationError> {
        let oracle_collision = query
            .inline_ground_truth()
            .ok_or(EvaluationError::MissingOracleGroundTruth)?;
        Ok(Self::new(
            QueryIdentity::from_imported(dataset, scene, time_step, query),
            oracle_collision,
            candidate,
            narrowphase,
        ))
    }
}

/// A tested algorithm's result for one stable query identity. The independent
/// oracle label is joined later from the imported dataset, never supplied by the
/// algorithm output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryRunResult {
    identity: QueryIdentity,
    candidate: CandidateDisposition,
    narrowphase: NarrowphaseOutcome,
    narrowphase_elapsed_ns: Option<u64>,
}

impl QueryRunResult {
    #[must_use]
    pub fn new(
        identity: QueryIdentity,
        candidate: CandidateDisposition,
        narrowphase: NarrowphaseOutcome,
    ) -> Self {
        Self {
            identity,
            candidate,
            narrowphase,
            narrowphase_elapsed_ns: None,
        }
    }

    #[must_use]
    pub fn with_narrowphase_elapsed_ns(mut self, elapsed_ns: u64) -> Self {
        self.narrowphase_elapsed_ns = Some(elapsed_ns);
        self
    }

    #[must_use]
    pub fn from_imported(
        dataset: impl Into<String>,
        scene: impl Into<String>,
        time_step: u64,
        query: &CcdQuery,
        candidate: CandidateDisposition,
        narrowphase: NarrowphaseOutcome,
    ) -> Self {
        Self::new(
            QueryIdentity::from_imported(dataset, scene, time_step, query),
            candidate,
            narrowphase,
        )
    }

    #[must_use]
    pub fn identity(&self) -> &QueryIdentity {
        &self.identity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryRunImportError {
    EmptyInput,
    BlankRow,
    WrongFieldCount,
    InvalidIdentityToken,
    InvalidTimeStep,
    InvalidQueryIndex,
    InvalidQueryClass,
    InvalidCandidateDisposition,
    InvalidFallbackReason,
    MissingFallbackReason,
    UnexpectedDispositionMetadata,
    MissingPruneEvidence,
    InvalidVerificationStatus,
    InvalidNarrowphaseOutcome,
    MissingNarrowphaseDuration,
    InvalidNarrowphaseDuration,
    UnexpectedNarrowphaseDuration,
}

impl fmt::Display for QueryRunImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for QueryRunImportError {}

/// Parse the strict tab-separated per-query result interchange format.
///
/// Optional first line: `#ccd-query-run-results-v1`.
/// Each data row has twelve fields:
/// dataset, scene, timestep, query class, query index, disposition,
/// fallback reason, certificate SHA-256, frozen-input SHA-256, verifier ID,
/// verification status, narrowphase outcome.
///
/// Forwarded rows must leave fields 6-10 empty. Fallback rows require only a
/// reason in field 6. Certified-pruned rows require all proof fields 7-10 and
/// no fallback reason. Oracle labels are intentionally absent from this file;
/// they are joined only from the independently imported dataset.
pub fn parse_query_run_results_tsv(
    input: &str,
) -> Result<Vec<QueryRunResult>, QueryRunImportError> {
    if input.trim().is_empty() {
        return Err(QueryRunImportError::EmptyInput);
    }

    let mut results = Vec::new();
    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line_index == 0 && line == "#ccd-query-run-results-v1" {
            continue;
        }
        if line.trim().is_empty() {
            return Err(QueryRunImportError::BlankRow);
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 13 {
            return Err(QueryRunImportError::WrongFieldCount);
        }

        if !valid_identity_token(fields[0]) || !valid_identity_token(fields[1]) {
            return Err(QueryRunImportError::InvalidIdentityToken);
        }
        let time_step = fields[2]
            .parse::<u64>()
            .map_err(|_| QueryRunImportError::InvalidTimeStep)?;
        let query_index = fields[4]
            .parse::<usize>()
            .map_err(|_| QueryRunImportError::InvalidQueryIndex)?;
        let query_class = match fields[3] {
            "edge_edge" => QueryClass::EdgeEdge,
            "vertex_face" => QueryClass::VertexFace,
            _ => return Err(QueryRunImportError::InvalidQueryClass),
        };
        let identity =
            QueryIdentity::new(fields[0], fields[1], time_step, query_class, query_index);

        let candidate = match fields[5] {
            "forwarded" => {
                if fields[6..11].iter().any(|field| !field.is_empty()) {
                    return Err(QueryRunImportError::UnexpectedDispositionMetadata);
                }
                CandidateDisposition::Forwarded
            }
            "fallback" => {
                if fields[6].is_empty() {
                    return Err(QueryRunImportError::MissingFallbackReason);
                }
                if fields[7..11].iter().any(|field| !field.is_empty()) {
                    return Err(QueryRunImportError::UnexpectedDispositionMetadata);
                }
                CandidateDisposition::Fallback(parse_fallback_reason(fields[6])?)
            }
            "certified_pruned" => {
                if !fields[6].is_empty() {
                    return Err(QueryRunImportError::UnexpectedDispositionMetadata);
                }
                if fields[7..11].iter().any(|field| field.is_empty()) {
                    return Err(QueryRunImportError::MissingPruneEvidence);
                }
                let status = match fields[10] {
                    "accepted_and_replayed" => VerificationStatus::AcceptedAndReplayed,
                    "rejected" => VerificationStatus::Rejected,
                    "not_run" => VerificationStatus::NotRun,
                    _ => return Err(QueryRunImportError::InvalidVerificationStatus),
                };
                CandidateDisposition::CertifiedPruned(PruneEvidence::new(
                    fields[7], fields[8], fields[9], status,
                ))
            }
            _ => return Err(QueryRunImportError::InvalidCandidateDisposition),
        };

        let narrowphase = match fields[11] {
            "collision" => NarrowphaseOutcome::Collision,
            "no_collision" => NarrowphaseOutcome::NoCollision,
            "indeterminate" => NarrowphaseOutcome::Indeterminate,
            "not_run" => NarrowphaseOutcome::NotRun,
            _ => return Err(QueryRunImportError::InvalidNarrowphaseOutcome),
        };
        let elapsed_ns = if narrowphase == NarrowphaseOutcome::NotRun {
            if !fields[12].is_empty() {
                return Err(QueryRunImportError::UnexpectedNarrowphaseDuration);
            }
            None
        } else {
            if fields[12].is_empty() {
                return Err(QueryRunImportError::MissingNarrowphaseDuration);
            }
            let elapsed = fields[12]
                .parse::<u64>()
                .map_err(|_| QueryRunImportError::InvalidNarrowphaseDuration)?;
            if elapsed == 0 {
                return Err(QueryRunImportError::InvalidNarrowphaseDuration);
            }
            Some(elapsed)
        };

        let result = QueryRunResult::new(identity, candidate, narrowphase);
        results.push(match elapsed_ns {
            Some(elapsed) => result.with_narrowphase_elapsed_ns(elapsed),
            None => result,
        });
    }

    if results.is_empty() {
        Err(QueryRunImportError::EmptyInput)
    } else {
        Ok(results)
    }
}

fn parse_fallback_reason(value: &str) -> Result<FallbackReason, QueryRunImportError> {
    match value {
        "unsupported_motion" => Ok(FallbackReason::UnsupportedMotion),
        "unknown_numeric_profile" => Ok(FallbackReason::UnknownNumericProfile),
        "invalid_input" => Ok(FallbackReason::InvalidInput),
        "arithmetic_overflow" => Ok(FallbackReason::ArithmeticOverflow),
        "rejected_certificate" => Ok(FallbackReason::RejectedCertificate),
        "missing_proof" => Ok(FallbackReason::MissingProof),
        "capacity_limit" => Ok(FallbackReason::CapacityLimit),
        _ => Err(QueryRunImportError::InvalidFallbackReason),
    }
}

fn valid_identity_token(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryJoinError {
    DuplicateRunResultIdentity,
    DuplicateImportedQueryIdentity,
    MissingRunResult,
    UnmatchedRunResult,
    MissingOracleGroundTruth,
    Evaluation(EvaluationError),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueryCounts {
    pub total_queries: u64,
    pub oracle_positive_queries: u64,
    pub oracle_negative_queries: u64,
    /// Forwarded plus fallback queries; both reach narrowphase.
    pub candidate_forwarded_queries: u64,
    pub candidate_true_positives: u64,
    pub candidate_false_positives: u64,
    pub candidate_false_negatives: u64,
    pub certified_pruned_queries: u64,
    pub certified_pruned_oracle_negatives: u64,
    pub fallback_queries: u64,
    pub fallback_oracle_positives: u64,
    pub narrowphase_collision_queries: u64,
    pub narrowphase_true_positives: u64,
    /// Oracle-positive forwarded/fallback queries eligible for narrowphase.
    pub narrowphase_eligible_positive_queries: u64,
    pub narrowphase_false_positives: u64,
    /// Oracle-positive queries not reported as a collision by narrowphase.
    pub narrowphase_false_negatives: u64,
    pub narrowphase_indeterminate_queries: u64,
    pub narrowphase_not_run_queries: u64,
    pub accepted_replayed_prune_certificates: u64,
}

impl QueryCounts {
    #[must_use]
    pub fn candidate_recall_basis_points(&self) -> Option<u64> {
        rate_basis_points(self.candidate_true_positives, self.oracle_positive_queries)
    }

    #[must_use]
    pub fn candidate_precision_basis_points(&self) -> Option<u64> {
        rate_basis_points(
            self.candidate_true_positives,
            self.candidate_forwarded_queries,
        )
    }

    #[must_use]
    pub fn candidate_false_positive_rate_basis_points(&self) -> Option<u64> {
        rate_basis_points(self.candidate_false_positives, self.oracle_negative_queries)
    }

    /// Conditional narrowphase recall: denominator includes only oracle-positive
    /// queries that actually reached narrowphase. Candidate-pruned positives
    /// are counted separately in candidate_false_negatives.
    #[must_use]
    pub fn narrowphase_recall_basis_points(&self) -> Option<u64> {
        rate_basis_points(
            self.narrowphase_true_positives,
            self.narrowphase_eligible_positive_queries,
        )
    }

    /// End-to-end recall across all oracle-positive queries; candidate misses
    /// and narrowphase misses both reduce this score.
    #[must_use]
    pub fn end_to_end_recall_basis_points(&self) -> Option<u64> {
        rate_basis_points(
            self.narrowphase_true_positives,
            self.oracle_positive_queries,
        )
    }

    #[must_use]
    pub fn end_to_end_false_negatives(&self) -> u64 {
        self.candidate_false_negatives + self.narrowphase_false_negatives
    }

    #[must_use]
    pub fn narrowphase_precision_basis_points(&self) -> Option<u64> {
        rate_basis_points(
            self.narrowphase_true_positives,
            self.narrowphase_collision_queries,
        )
    }

    #[must_use]
    pub fn fallback_rate_basis_points(&self) -> Option<u64> {
        rate_basis_points(self.fallback_queries, self.total_queries)
    }

    /// Safe negative prunes do not disqualify a score simply because their
    /// narrowphase was correctly not run.
    #[must_use]
    pub fn correctness_qualified(&self) -> bool {
        self.candidate_false_negatives == 0
            && self.narrowphase_false_negatives == 0
            && self.narrowphase_indeterminate_queries == 0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct QueryStratumIdentity {
    dataset: String,
    scene: String,
    time_step: u64,
    query_class: QueryClass,
}

impl QueryStratumIdentity {
    #[must_use]
    pub fn dataset(&self) -> &str {
        &self.dataset
    }

    #[must_use]
    pub fn scene(&self) -> &str {
        &self.scene
    }

    #[must_use]
    pub fn time_step(&self) -> u64 {
        self.time_step
    }

    #[must_use]
    pub fn query_class(&self) -> QueryClass {
        self.query_class
    }
}

/// Nearest-rank p50/p95/p99 of per-query narrowphase durations.
/// Durations are integer nanoseconds; empty populations remain unavailable.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueryTimingSummary {
    pub sample_count: u64,
    pub p50_ns: Option<u64>,
    pub p95_ns: Option<u64>,
    pub p99_ns: Option<u64>,
}

impl QueryTimingSummary {
    #[must_use]
    pub fn complete_for(&self, expected_samples: u64) -> bool {
        self.sample_count == expected_samples
            && (expected_samples == 0
                || (self.p50_ns.is_some() && self.p95_ns.is_some() && self.p99_ns.is_some()))
    }
}

fn summarize_query_durations(mut durations_ns: Vec<u64>) -> QueryTimingSummary {
    if durations_ns.is_empty() {
        return QueryTimingSummary::default();
    }
    durations_ns.sort_unstable();
    let nearest_rank = |percentile: u64| {
        let rank = ((durations_ns.len() as u128 * u128::from(percentile)).div_ceil(100))
            .max(1) as usize;
        durations_ns[rank - 1]
    };
    QueryTimingSummary {
        sample_count: durations_ns.len() as u64,
        p50_ns: Some(nearest_rank(50)),
        p95_ns: Some(nearest_rank(95)),
        p99_ns: Some(nearest_rank(99)),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryStratumScore {
    pub identity: QueryStratumIdentity,
    pub counts: QueryCounts,
    pub timing: QueryTimingSummary,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MotionProfile {
    LinearizedConstantVelocity,
    SemiImplicitEulerNative,
    SemiImplicitEulerQ16_16,
    GeneralRigidBodyRotation,
    Unspecified,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArithmeticProfile {
    Binary64,
    ExactRational,
    OutwardIntervalBinary64,
    FixedQ16_16,
    MixedPrecision,
    Unspecified,
}

/// Immutable provenance required to reproduce and pool one benchmark run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunProvenance {
    algorithm_name: String,
    algorithm_version: String,
    algorithm_commit_sha1: String,
    dataset_manifest_sha256: String,
    configuration_sha256: String,
}

impl RunProvenance {
    pub fn new(
        algorithm_name: impl Into<String>,
        algorithm_version: impl Into<String>,
        algorithm_commit_sha1: impl Into<String>,
        dataset_manifest_sha256: impl Into<String>,
        configuration_sha256: impl Into<String>,
    ) -> Self {
        Self {
            algorithm_name: algorithm_name.into(),
            algorithm_version: algorithm_version.into(),
            algorithm_commit_sha1: algorithm_commit_sha1.into(),
            dataset_manifest_sha256: dataset_manifest_sha256.into(),
            configuration_sha256: configuration_sha256.into(),
        }
    }

    #[must_use]
    pub fn algorithm_name(&self) -> &str {
        &self.algorithm_name
    }

    #[must_use]
    pub fn algorithm_version(&self) -> &str {
        &self.algorithm_version
    }

    #[must_use]
    pub fn algorithm_commit_sha1(&self) -> &str {
        &self.algorithm_commit_sha1
    }

    #[must_use]
    pub fn dataset_manifest_sha256(&self) -> &str {
        &self.dataset_manifest_sha256
    }

    #[must_use]
    pub fn configuration_sha256(&self) -> &str {
        &self.configuration_sha256
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.algorithm_name.trim().is_empty()
            && !self.algorithm_version.trim().is_empty()
            && is_lower_hex(&self.algorithm_commit_sha1, 40)
            && is_lower_hex(&self.dataset_manifest_sha256, 64)
            && is_lower_hex(&self.configuration_sha256, 64)
    }
}

/// One query scorecard represents exactly one immutable algorithm, dataset,
/// configuration, motion and arithmetic profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunProfile {
    provenance: Option<RunProvenance>,
    motion: MotionProfile,
    arithmetic: ArithmeticProfile,
}

impl RunProfile {
    pub const UNSPECIFIED: Self = Self {
        provenance: None,
        motion: MotionProfile::Unspecified,
        arithmetic: ArithmeticProfile::Unspecified,
    };

    #[must_use]
    pub fn new(
        provenance: RunProvenance,
        motion: MotionProfile,
        arithmetic: ArithmeticProfile,
    ) -> Self {
        Self {
            provenance: Some(provenance),
            motion,
            arithmetic,
        }
    }

    #[must_use]
    pub fn provenance(&self) -> Option<&RunProvenance> {
        self.provenance.as_ref()
    }

    #[must_use]
    pub fn motion(&self) -> MotionProfile {
        self.motion
    }

    #[must_use]
    pub fn arithmetic(&self) -> ArithmeticProfile {
        self.arithmetic
    }

    #[must_use]
    pub fn is_explicit(&self) -> bool {
        self.provenance
            .as_ref()
            .is_some_and(RunProvenance::is_valid)
            && self.motion != MotionProfile::Unspecified
            && self.arithmetic != ArithmeticProfile::Unspecified
    }

    /// Pooling is permitted only for identical algorithm commit, dataset
    /// manifest, configuration, motion and arithmetic provenance.
    #[must_use]
    pub fn can_pool_with(&self, other: &Self) -> bool {
        self.is_explicit() && other.is_explicit() && self == other
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryScore {
    /// Exact algorithm, dataset, configuration, motion and arithmetic profile.
    pub profile: RunProfile,
    /// Micro-aggregated counts over the full evaluated query population.
    pub counts: QueryCounts,
    /// Per-query narrowphase latency across forwarded and fallback queries.
    pub timing: QueryTimingSummary,
    /// Independently reported per dataset/scene/time-step/query-kind strata.
    pub strata: Vec<QueryStratumScore>,
}

impl QueryScore {
    #[must_use]
    pub fn benchmark_qualified(&self) -> bool {
        self.profile.is_explicit()
            && self.counts.oracle_positive_queries > 0
            && self.counts.correctness_qualified()
            && self
                .timing
                .complete_for(self.counts.candidate_forwarded_queries)
    }

    #[must_use]
    pub fn can_pool_with(&self, other: &Self) -> bool {
        self.profile.can_pool_with(&other.profile)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    EmptyDatasetOrScene,
    DuplicateQueryIdentity,
    InvalidPruneCertificateDigest,
    EmptyPruneVerifierIdentity,
    PruneCertificateNotAcceptedAndReplayed,
    CertifiedPruneReachedNarrowphase,
    ForwardedQueryNotRunInNarrowphase,
    MissingOracleGroundTruth,
    ZeroNarrowphaseDuration,
    DurationForNotRunNarrowphase,
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for EvaluationError {}

/// Validate query dispositions and compute aggregate plus stratified scores.
///
/// Query metrics are not broadphase box-pair or Symtropy body/collider-pair
/// metrics. Those require independent identity-preserving populations.
/// Convenience evaluator for metric/unit tests. Its profile is deliberately
/// unspecified, so the result is not eligible for benchmark pooling or release.
pub fn evaluate_queries(
    evaluations: &[QueryEvaluation],
) -> Result<QueryScore, Vec<EvaluationError>> {
    evaluate_queries_with_profile(RunProfile::UNSPECIFIED, evaluations)
}

/// Evaluate one run under an explicitly declared motion and arithmetic model.
pub fn evaluate_queries_with_profile(
    profile: RunProfile,
    evaluations: &[QueryEvaluation],
) -> Result<QueryScore, Vec<EvaluationError>> {
    let mut errors = Vec::new();
    let mut identities = HashSet::with_capacity(evaluations.len());
    let mut strata = std::collections::BTreeMap::<QueryStratumIdentity, QueryCounts>::new();
    let mut all_durations_ns = Vec::new();
    let mut stratum_durations_ns =
        std::collections::BTreeMap::<QueryStratumIdentity, Vec<u64>>::new();
    let mut counts = QueryCounts::default();

    for evaluation in evaluations {
        let identity = &evaluation.identity;
        if !identity.valid() {
            errors.push(EvaluationError::EmptyDatasetOrScene);
        }
        if !identities.insert(identity.clone()) {
            errors.push(EvaluationError::DuplicateQueryIdentity);
        }

        if let CandidateDisposition::CertifiedPruned(evidence) = &evaluation.candidate {
            if evidence.status != VerificationStatus::AcceptedAndReplayed {
                errors.push(EvaluationError::PruneCertificateNotAcceptedAndReplayed);
            }
            if !is_lower_hex(&evidence.certificate_sha256, 64)
                || !is_lower_hex(&evidence.frozen_input_sha256, 64)
            {
                errors.push(EvaluationError::InvalidPruneCertificateDigest);
            }
            if evidence.verifier_id.trim().is_empty() {
                errors.push(EvaluationError::EmptyPruneVerifierIdentity);
            }
        }

        let forwarded = !matches!(
            &evaluation.candidate,
            CandidateDisposition::CertifiedPruned(_)
        );

        if forwarded && evaluation.narrowphase == NarrowphaseOutcome::NotRun {
            errors.push(EvaluationError::ForwardedQueryNotRunInNarrowphase);
        }
        if !forwarded && evaluation.narrowphase != NarrowphaseOutcome::NotRun {
            errors.push(EvaluationError::CertifiedPruneReachedNarrowphase);
        }

        accumulate_counts(&mut counts, evaluation, forwarded);
        let stratum_identity = QueryStratumIdentity {
            dataset: identity.dataset.clone(),
            scene: identity.scene.clone(),
            time_step: identity.time_step,
            query_class: identity.query_class,
        };
        accumulate_counts(
            strata.entry(stratum_identity.clone()).or_default(),
            evaluation,
            forwarded,
        );
        if let Some(elapsed_ns) = evaluation.narrowphase_elapsed_ns {
            if elapsed_ns == 0 {
                errors.push(EvaluationError::ZeroNarrowphaseDuration);
            } else if evaluation.narrowphase == NarrowphaseOutcome::NotRun {
                errors.push(EvaluationError::DurationForNotRunNarrowphase);
            } else {
                all_durations_ns.push(elapsed_ns);
                stratum_durations_ns
                    .entry(stratum_identity)
                    .or_default()
                    .push(elapsed_ns);
            }
        }
    }

    if errors.is_empty() {
        let timing = summarize_query_durations(all_durations_ns);
        let strata = strata
            .into_iter()
            .map(|(identity, counts)| {
                let timing = summarize_query_durations(
                    stratum_durations_ns.remove(&identity).unwrap_or_default(),
                );
                QueryStratumScore {
                    identity,
                    counts,
                    timing,
                }
            })
            .collect();
        Ok(QueryScore {
            profile,
            counts,
            timing,
            strata,
        })
    } else {
        Err(errors)
    }
}

/// Join algorithm output to imported oracle queries by full stable identity.
///
/// The input query list is the denominator. Missing results, duplicate result
/// IDs and orphan results are hard errors. Record ordering is irrelevant.
/// Convenience join for unit tests and profile-independent diagnostics. It
/// returns an unspecified profile and cannot be pooled as a benchmark score.
pub fn evaluate_imported_queries(
    dataset: impl Into<String>,
    scene: impl Into<String>,
    time_step: u64,
    queries: &[CcdQuery],
    run_results: &[QueryRunResult],
) -> Result<QueryScore, Vec<QueryJoinError>> {
    evaluate_imported_queries_with_profile(
        RunProfile::UNSPECIFIED,
        dataset,
        scene,
        time_step,
        queries,
        run_results,
    )
}

/// Join algorithm output by stable query identity and qualify it under one
/// explicit motion/arithmetic profile.
pub fn evaluate_imported_queries_with_profile(
    profile: RunProfile,
    dataset: impl Into<String>,
    scene: impl Into<String>,
    time_step: u64,
    queries: &[CcdQuery],
    run_results: &[QueryRunResult],
) -> Result<QueryScore, Vec<QueryJoinError>> {
    let dataset = dataset.into();
    let scene = scene.into();
    let mut by_identity = HashMap::with_capacity(run_results.len());
    let mut errors = Vec::new();

    for result in run_results {
        if by_identity
            .insert(result.identity.clone(), result)
            .is_some()
        {
            errors.push(QueryJoinError::DuplicateRunResultIdentity);
        }
    }

    let mut consumed = HashSet::with_capacity(queries.len());
    let mut evaluations = Vec::with_capacity(queries.len());
    for query in queries {
        let identity = QueryIdentity::from_imported(&dataset, &scene, time_step, query);
        if !consumed.insert(identity.clone()) {
            errors.push(QueryJoinError::DuplicateImportedQueryIdentity);
            continue;
        }
        let Some(result) = by_identity.get(&identity) else {
            errors.push(QueryJoinError::MissingRunResult);
            continue;
        };
        let Some(oracle_collision) = query.inline_ground_truth() else {
            errors.push(QueryJoinError::MissingOracleGroundTruth);
            continue;
        };
        let evaluation = QueryEvaluation::new(
            identity,
            oracle_collision,
            result.candidate.clone(),
            result.narrowphase,
        );
        evaluations.push(match result.narrowphase_elapsed_ns {
            Some(elapsed_ns) => evaluation.with_narrowphase_elapsed_ns(elapsed_ns),
            None => evaluation,
        });
    }

    if by_identity
        .keys()
        .any(|identity| !consumed.contains(identity))
    {
        errors.push(QueryJoinError::UnmatchedRunResult);
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    evaluate_queries_with_profile(profile, &evaluations).map_err(|evaluation_errors| {
        evaluation_errors
            .into_iter()
            .map(QueryJoinError::Evaluation)
            .collect()
    })
}

fn accumulate_counts(counts: &mut QueryCounts, evaluation: &QueryEvaluation, forwarded: bool) {
    counts.total_queries += 1;
    if evaluation.oracle_collision {
        counts.oracle_positive_queries += 1;
    } else {
        counts.oracle_negative_queries += 1;
    }

    if forwarded {
        counts.candidate_forwarded_queries += 1;
        if evaluation.oracle_collision {
            counts.candidate_true_positives += 1;
        } else {
            counts.candidate_false_positives += 1;
        }
    } else {
        counts.certified_pruned_queries += 1;
        if evaluation.oracle_collision {
            counts.candidate_false_negatives += 1;
        } else {
            counts.certified_pruned_oracle_negatives += 1;
        }
    }

    if matches!(&evaluation.candidate, CandidateDisposition::Fallback(_)) {
        counts.fallback_queries += 1;
        if evaluation.oracle_collision {
            counts.fallback_oracle_positives += 1;
        }
    }

    // Narrowphase-stage denominators contain only positives that were actually
    // forwarded (including fallback), not positives already pruned upstream.
    if forwarded && evaluation.oracle_collision {
        counts.narrowphase_eligible_positive_queries += 1;
    }

    match evaluation.narrowphase {
        NarrowphaseOutcome::Collision => {
            counts.narrowphase_collision_queries += 1;
            if evaluation.oracle_collision {
                counts.narrowphase_true_positives += 1;
            } else {
                counts.narrowphase_false_positives += 1;
            }
        }
        NarrowphaseOutcome::NoCollision => {
            if evaluation.oracle_collision {
                counts.narrowphase_false_negatives += 1;
            }
        }
        NarrowphaseOutcome::Indeterminate => {
            counts.narrowphase_indeterminate_queries += 1;
            if evaluation.oracle_collision {
                counts.narrowphase_false_negatives += 1;
            }
        }
        NarrowphaseOutcome::NotRun => {
            counts.narrowphase_not_run_queries += 1;
        }
    }

    if matches!(
        &evaluation.candidate,
        CandidateDisposition::CertifiedPruned(PruneEvidence {
            status: VerificationStatus::AcceptedAndReplayed,
            ..
        })
    ) && evaluation.narrowphase == NarrowphaseOutcome::NotRun
    {
        counts.accepted_replayed_prune_certificates += 1;
    }
}

fn rate_basis_points(numerator: u64, denominator: u64) -> Option<u64> {
    if denominator == 0 {
        return None;
    }
    Some((u128::from(numerator) * 10_000 / u128::from(denominator)) as u64)
}

fn is_lower_hex(value: &str, expected_length: usize) -> bool {
    value.len() == expected_length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nyu_ccd_query_import::{
        QueryKind, attach_ground_truth, parse_boolean_json, parse_queries_csv,
    };

    fn run_profile(
        algorithm_name: &str,
        algorithm_commit: &str,
        motion: MotionProfile,
        arithmetic: ArithmeticProfile,
    ) -> RunProfile {
        RunProfile::new(
            RunProvenance::new(
                algorithm_name,
                "v0.1",
                algorithm_commit,
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            ),
            motion,
            arithmetic,
        )
    }

    fn evidence(status: VerificationStatus) -> PruneEvidence {
        PruneEvidence::new(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "symtropy-aabb-checker-v0.1",
            status,
        )
    }

    fn imported_queries_with_labels(labels: &[bool]) -> Vec<CcdQuery> {
        let mut csv = String::new();
        for (query_index, label) in labels.iter().copied().enumerate() {
            for vertex in 0..8 {
                if !csv.is_empty() {
                    csv.push('\n');
                }
                csv.push_str(&format!(
                    "{},{},0,1,0,1,{}",
                    query_index * 8 + vertex + 1,
                    1,
                    if label { "true" } else { "false" }
                ));
            }
        }
        parse_queries_csv(&csv, QueryKind::EdgeEdge).unwrap()
    }

    fn evaluation(
        query: &CcdQuery,
        scene: &str,
        candidate: CandidateDisposition,
        narrowphase: NarrowphaseOutcome,
    ) -> QueryEvaluation {
        QueryEvaluation::from_imported("nyu-fixture-v1", scene, 3, query, candidate, narrowphase)
            .unwrap()
    }

    #[test]
    fn benchmark_scores_require_and_preserve_explicit_motion_precision_profiles() {
        let queries = imported_queries_with_labels(&[true]);
        let result = evaluation(
            &queries[0],
            "profile-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::Collision,
        );
        let binary64 = run_profile(
            "symtropy",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            MotionProfile::LinearizedConstantVelocity,
            ArithmeticProfile::Binary64,
        );
        let q16 = run_profile(
            "symtropy",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            MotionProfile::SemiImplicitEulerQ16_16,
            ArithmeticProfile::FixedQ16_16,
        );
        let other_algorithm = run_profile(
            "ccd-wrapper",
            "dddddddddddddddddddddddddddddddddddddddd",
            MotionProfile::LinearizedConstantVelocity,
            ArithmeticProfile::Binary64,
        );
        let binary_score =
            evaluate_queries_with_profile(binary64, std::slice::from_ref(&result)).unwrap();
        let q16_score = evaluate_queries_with_profile(q16, std::slice::from_ref(&result)).unwrap();
        let unspecified = evaluate_queries(std::slice::from_ref(&result)).unwrap();

        assert!(binary_score.benchmark_qualified());
        assert!(q16_score.benchmark_qualified());
        assert!(binary_score.can_pool_with(&binary_score));
        assert!(!binary_score.can_pool_with(&q16_score));
        let other_score =
            evaluate_queries_with_profile(other_algorithm, std::slice::from_ref(&result)).unwrap();
        assert!(!binary_score.can_pool_with(&other_score));
        assert!(!unspecified.benchmark_qualified());
        assert!(!unspecified.can_pool_with(&binary_score));
    }

    #[test]
    fn zero_positive_population_cannot_claim_benchmark_recall() {
        let queries = imported_queries_with_labels(&[false]);
        let result = evaluation(
            &queries[0],
            "no-positive-profile-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );
        let profile = run_profile(
            "symtropy",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            MotionProfile::LinearizedConstantVelocity,
            ArithmeticProfile::Binary64,
        );
        let score = evaluate_queries_with_profile(profile, &[result]).unwrap();
        assert!(score.counts.correctness_qualified());
        assert!(!score.benchmark_qualified());
    }

    #[test]
    fn real_armadillo_sample_keeps_zero_positive_recall_unavailable() {
        const CSV: &str =
            include_str!("../../../nyu-ccd-query-import/v0.1/fixtures/armadillo-rollers-3ee.csv");
        const BOOLS: &str = include_str!(
            "../../../nyu-ccd-query-import/v0.1/fixtures/armadillo-rollers-3ee-mma-bool.json"
        );
        let mut queries = parse_queries_csv(CSV, QueryKind::EdgeEdge).unwrap();
        attach_ground_truth(&mut queries, &parse_boolean_json(BOOLS).unwrap()).unwrap();

        let results = queries
            .iter()
            .enumerate()
            .map(|(index, query)| {
                if index == 0 {
                    evaluation(
                        query,
                        "armadillo-rollers",
                        CandidateDisposition::CertifiedPruned(evidence(
                            VerificationStatus::AcceptedAndReplayed,
                        )),
                        NarrowphaseOutcome::NotRun,
                    )
                } else {
                    evaluation(
                        query,
                        "armadillo-rollers",
                        CandidateDisposition::Forwarded,
                        NarrowphaseOutcome::NoCollision,
                    )
                }
            })
            .collect::<Vec<_>>();
        let score = evaluate_queries(&results).unwrap();

        assert_eq!(score.counts.total_queries, 2);
        assert_eq!(score.counts.oracle_positive_queries, 0);
        assert_eq!(score.counts.oracle_negative_queries, 2);
        assert_eq!(score.counts.candidate_recall_basis_points(), None);
        assert_eq!(score.counts.narrowphase_recall_basis_points(), None);
        assert_eq!(score.counts.candidate_false_positives, 1);
        assert_eq!(score.counts.certified_pruned_queries, 1);
        assert_eq!(score.counts.accepted_replayed_prune_certificates, 1);
        assert!(score.counts.correctness_qualified());
    }

    #[test]
    fn separates_candidate_and_narrowphase_false_negatives() {
        let queries = imported_queries_with_labels(&[true, true, false, false]);
        let outcomes = vec![
            evaluation(
                &queries[0],
                "scene-a",
                CandidateDisposition::Forwarded,
                NarrowphaseOutcome::Collision,
            ),
            evaluation(
                &queries[1],
                "scene-a",
                CandidateDisposition::CertifiedPruned(evidence(
                    VerificationStatus::AcceptedAndReplayed,
                )),
                NarrowphaseOutcome::NotRun,
            ),
            evaluation(
                &queries[2],
                "scene-a",
                CandidateDisposition::Forwarded,
                NarrowphaseOutcome::NoCollision,
            ),
            evaluation(
                &queries[3],
                "scene-a",
                CandidateDisposition::CertifiedPruned(evidence(
                    VerificationStatus::AcceptedAndReplayed,
                )),
                NarrowphaseOutcome::NotRun,
            ),
        ];
        let score = evaluate_queries(&outcomes).unwrap();
        let counts = score.counts;

        assert_eq!(counts.oracle_positive_queries, 2);
        assert_eq!(counts.oracle_negative_queries, 2);
        assert_eq!(counts.candidate_forwarded_queries, 2);
        assert_eq!(counts.candidate_true_positives, 1);
        assert_eq!(counts.candidate_false_positives, 1);
        assert_eq!(counts.candidate_false_negatives, 1);
        assert_eq!(counts.candidate_recall_basis_points(), Some(5_000));
        assert_eq!(counts.narrowphase_true_positives, 1);
        assert_eq!(counts.narrowphase_eligible_positive_queries, 1);
        assert_eq!(counts.narrowphase_false_negatives, 0);
        assert_eq!(counts.narrowphase_recall_basis_points(), Some(10_000));
        assert_eq!(counts.end_to_end_recall_basis_points(), Some(5_000));
        assert_eq!(counts.end_to_end_false_negatives(), 1);
        assert_eq!(counts.certified_pruned_oracle_negatives, 1);
        assert_eq!(counts.accepted_replayed_prune_certificates, 2);
        assert!(!counts.correctness_qualified());
    }

    #[test]
    fn forwarded_oracle_positive_with_narrowphase_miss_is_not_a_candidate_miss() {
        let queries = imported_queries_with_labels(&[true]);
        let result = evaluation(
            &queries[0],
            "narrowphase-miss-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );
        let score = evaluate_queries(&[result]).unwrap();

        assert_eq!(score.counts.candidate_false_negatives, 0);
        assert_eq!(score.counts.candidate_recall_basis_points(), Some(10_000));
        assert_eq!(score.counts.narrowphase_eligible_positive_queries, 1);
        assert_eq!(score.counts.narrowphase_false_negatives, 1);
        assert_eq!(score.counts.narrowphase_recall_basis_points(), Some(0));
        assert_eq!(score.counts.end_to_end_recall_basis_points(), Some(0));
        assert_eq!(score.counts.end_to_end_false_negatives(), 1);
        assert!(!score.counts.correctness_qualified());
    }

    #[test]
    fn safe_negative_certificate_prune_can_still_qualify() {
        let queries = imported_queries_with_labels(&[false]);
        let result = evaluation(
            &queries[0],
            "safe-negative-prune-scene",
            CandidateDisposition::CertifiedPruned(evidence(
                VerificationStatus::AcceptedAndReplayed,
            )),
            NarrowphaseOutcome::NotRun,
        );
        let score = evaluate_queries(&[result]).unwrap();

        assert_eq!(score.counts.candidate_false_negatives, 0);
        assert_eq!(score.counts.narrowphase_false_negatives, 0);
        assert_eq!(score.counts.narrowphase_not_run_queries, 1);
        assert!(score.counts.correctness_qualified());
    }

    #[test]
    fn fallback_is_forwarded_and_never_credited_as_a_prune() {
        let queries = imported_queries_with_labels(&[true, false]);
        let outcomes = vec![
            evaluation(
                &queries[0],
                "fallback-scene",
                CandidateDisposition::Fallback(FallbackReason::UnsupportedMotion),
                NarrowphaseOutcome::Collision,
            ),
            evaluation(
                &queries[1],
                "fallback-scene",
                CandidateDisposition::Fallback(FallbackReason::UnknownNumericProfile),
                NarrowphaseOutcome::NoCollision,
            ),
        ];
        let score = evaluate_queries(&outcomes).unwrap();
        assert_eq!(score.counts.fallback_queries, 2);
        assert_eq!(score.counts.candidate_forwarded_queries, 2);
        assert_eq!(score.counts.certified_pruned_queries, 0);
        assert_eq!(score.counts.fallback_rate_basis_points(), Some(10_000));
        assert!(score.counts.correctness_qualified());
    }

    #[test]
    fn rejects_duplicate_query_identity() {
        let queries = imported_queries_with_labels(&[false]);
        let first = evaluation(
            &queries[0],
            "same-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );
        assert_eq!(
            evaluate_queries(&[first.clone(), first]),
            Err(vec![EvaluationError::DuplicateQueryIdentity])
        );
    }

    #[test]
    fn rejects_unreplayed_or_bad_prune_certificates() {
        let queries = imported_queries_with_labels(&[false]);
        let rejected = evaluation(
            &queries[0],
            "proof-scene",
            CandidateDisposition::CertifiedPruned(evidence(VerificationStatus::Rejected)),
            NarrowphaseOutcome::NotRun,
        );
        assert_eq!(
            evaluate_queries(&[rejected]),
            Err(vec![
                EvaluationError::PruneCertificateNotAcceptedAndReplayed
            ])
        );

        let malformed = QueryEvaluation::new(
            QueryIdentity::new("nyu-fixture-v1", "proof-scene", 3, QueryClass::EdgeEdge, 77),
            false,
            CandidateDisposition::CertifiedPruned(PruneEvidence::new(
                "not-a-digest",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "checker",
                VerificationStatus::AcceptedAndReplayed,
            )),
            NarrowphaseOutcome::NotRun,
        );
        assert_eq!(
            evaluate_queries(&[malformed]),
            Err(vec![EvaluationError::InvalidPruneCertificateDigest])
        );
    }

    #[test]
    fn distinguishes_indeterminate_from_collision_and_no_collision() {
        let queries = imported_queries_with_labels(&[true]);
        let outcome = evaluation(
            &queries[0],
            "indeterminate-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::Indeterminate,
        );
        let score = evaluate_queries(&[outcome]).unwrap();
        assert_eq!(score.counts.narrowphase_indeterminate_queries, 1);
        assert_eq!(score.counts.narrowphase_false_negatives, 1);
        assert!(!score.counts.correctness_qualified());
    }

    #[test]
    fn query_identity_includes_dataset_scene_step_kind_and_index() {
        let queries = imported_queries_with_labels(&[false]);
        let result = evaluation(
            &queries[0],
            "identity-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );
        let mut changed = result.clone();
        changed.identity.time_step += 1;
        assert_ne!(result.identity, changed.identity);
        changed = result.clone();
        changed.identity.query_class = QueryClass::VertexFace;
        assert_ne!(result.identity, changed.identity);
    }

    #[test]
    fn reports_independent_scene_frame_and_primitive_type_strata() {
        let edge_queries = imported_queries_with_labels(&[true, false]);
        let mut vertex_csv = String::new();
        for index in 0..8 {
            if !vertex_csv.is_empty() {
                vertex_csv.push(char::from(10));
            }
            vertex_csv.push_str(&format!("{index},1,0,1,0,1,false"));
        }
        let vertex_queries = parse_queries_csv(&vertex_csv, QueryKind::VertexFace).unwrap();

        let edge_positive = evaluation(
            &edge_queries[0],
            "shared-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::Collision,
        );
        let vertex_negative = evaluation(
            &vertex_queries[0],
            "shared-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );
        let mut next_frame_negative = evaluation(
            &edge_queries[1],
            "shared-scene",
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );
        next_frame_negative.identity.time_step = 4;

        let ordered = vec![
            edge_positive.clone(),
            vertex_negative.clone(),
            next_frame_negative.clone(),
        ];
        let score = evaluate_queries(&ordered).unwrap();
        let reversed =
            evaluate_queries(&[next_frame_negative, vertex_negative, edge_positive]).unwrap();

        assert_eq!(score, reversed);
        assert_eq!(score.strata.len(), 3);
        assert!(score.counts.correctness_qualified());
        assert_eq!(score.counts.total_queries, 3);
        assert_eq!(score.strata[0].identity.query_class(), QueryClass::EdgeEdge);
        assert_eq!(score.strata[0].identity.time_step(), 3);
        assert_eq!(score.strata[0].counts.oracle_positive_queries, 1);
    }

    #[test]
    fn joins_run_results_by_identity_independent_of_order() {
        const CSV: &str =
            include_str!("../../../nyu-ccd-query-import/v0.1/fixtures/armadillo-rollers-3ee.csv");
        const BOOLS: &str = include_str!(
            "../../../nyu-ccd-query-import/v0.1/fixtures/armadillo-rollers-3ee-mma-bool.json"
        );
        let mut queries = parse_queries_csv(CSV, QueryKind::EdgeEdge).unwrap();
        attach_ground_truth(&mut queries, &parse_boolean_json(BOOLS).unwrap()).unwrap();

        let results = queries
            .iter()
            .rev()
            .map(|query| {
                QueryRunResult::from_imported(
                    "nyu-fixture-v1",
                    "armadillo-rollers",
                    3,
                    query,
                    CandidateDisposition::Forwarded,
                    NarrowphaseOutcome::NoCollision,
                )
            })
            .collect::<Vec<_>>();
        let score =
            evaluate_imported_queries("nyu-fixture-v1", "armadillo-rollers", 3, &queries, &results)
                .unwrap();

        assert_eq!(score.counts.total_queries, 2);
        assert_eq!(score.counts.oracle_negative_queries, 2);
        assert_eq!(score.counts.candidate_false_positives, 2);
        let reversed_results = results.into_iter().rev().collect::<Vec<_>>();
        let reversed_score = evaluate_imported_queries(
            "nyu-fixture-v1",
            "armadillo-rollers",
            3,
            &queries,
            &reversed_results,
        )
        .unwrap();
        assert_eq!(score, reversed_score);
    }

    #[test]
    fn rejects_missing_duplicate_and_orphan_algorithm_results() {
        let queries = imported_queries_with_labels(&[false, false]);
        let first = QueryRunResult::from_imported(
            "nyu-fixture-v1",
            "adapter-scene",
            3,
            &queries[0],
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );
        let second = QueryRunResult::from_imported(
            "nyu-fixture-v1",
            "adapter-scene",
            3,
            &queries[1],
            CandidateDisposition::Forwarded,
            NarrowphaseOutcome::NoCollision,
        );

        let missing = evaluate_imported_queries(
            "nyu-fixture-v1",
            "adapter-scene",
            3,
            &queries,
            std::slice::from_ref(&first),
        )
        .unwrap_err();
        assert!(missing.contains(&QueryJoinError::MissingRunResult));

        let duplicate = evaluate_imported_queries(
            "nyu-fixture-v1",
            "adapter-scene",
            3,
            &queries,
            &[first.clone(), first.clone()],
        )
        .unwrap_err();
        assert!(duplicate.contains(&QueryJoinError::DuplicateRunResultIdentity));
        assert!(duplicate.contains(&QueryJoinError::MissingRunResult));

        let orphan = evaluate_imported_queries(
            "nyu-fixture-v1",
            "adapter-scene",
            3,
            &queries[..1],
            &[second],
        )
        .unwrap_err();
        assert!(orphan.contains(&QueryJoinError::MissingRunResult));
        assert!(orphan.contains(&QueryJoinError::UnmatchedRunResult));
    }

    #[test]
    fn parses_tsv_rows_for_forwarded_fallback_and_certified_prune() {
        let forwarded = [
            "nyu-fixture-v1",
            "armadillo-rollers",
            "3",
            "edge_edge",
            "0",
            "forwarded",
            "",
            "",
            "",
            "",
            "",
            "no_collision",
        ]
        .join("\t");
        let fallback = [
            "nyu-fixture-v1",
            "armadillo-rollers",
            "3",
            "edge_edge",
            "1",
            "fallback",
            "unsupported_motion",
            "",
            "",
            "",
            "",
            "no_collision",
        ]
        .join("\t");
        let pruned = [
            "nyu-fixture-v1",
            "armadillo-rollers",
            "3",
            "edge_edge",
            "2",
            "certified_pruned",
            "",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "symtropy-aabb-checker-v0.1",
            "accepted_and_replayed",
            "not_run",
        ]
        .join("\t");
        let input = ["#ccd-query-run-results-v1", &forwarded, &fallback, &pruned].join("\n");

        let results = parse_query_run_results_tsv(&input).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].identity().query_index(), 0);
        assert!(matches!(
            results[0].candidate,
            CandidateDisposition::Forwarded
        ));
        assert!(matches!(
            results[1].candidate,
            CandidateDisposition::Fallback(FallbackReason::UnsupportedMotion)
        ));
        assert!(matches!(
            results[2].candidate,
            CandidateDisposition::CertifiedPruned(PruneEvidence {
                status: VerificationStatus::AcceptedAndReplayed,
                ..
            })
        ));
    }

    #[test]
    fn tsv_adapter_fails_closed_on_malformed_dispositions_and_evidence() {
        let forwarded_with_metadata = [
            "dataset",
            "scene",
            "0",
            "edge_edge",
            "0",
            "forwarded",
            "unsupported_motion",
            "",
            "",
            "",
            "",
            "no_collision",
        ]
        .join("\t");
        assert_eq!(
            parse_query_run_results_tsv(&forwarded_with_metadata),
            Err(QueryRunImportError::UnexpectedDispositionMetadata)
        );

        let missing_fallback_reason = [
            "dataset",
            "scene",
            "0",
            "edge_edge",
            "0",
            "fallback",
            "",
            "",
            "",
            "",
            "",
            "no_collision",
        ]
        .join("\t");
        assert_eq!(
            parse_query_run_results_tsv(&missing_fallback_reason),
            Err(QueryRunImportError::MissingFallbackReason)
        );

        let missing_certificate = [
            "dataset",
            "scene",
            "0",
            "edge_edge",
            "0",
            "certified_pruned",
            "",
            "",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "checker",
            "accepted_and_replayed",
            "not_run",
        ]
        .join("\t");
        assert_eq!(
            parse_query_run_results_tsv(&missing_certificate),
            Err(QueryRunImportError::MissingPruneEvidence)
        );
    }

    #[test]
    fn tsv_adapter_joins_real_nyu_sample_to_profiled_query_score() {
        const CSV: &str =
            include_str!("../../../nyu-ccd-query-import/v0.1/fixtures/armadillo-rollers-3ee.csv");
        const BOOLS: &str = include_str!(
            "../../../nyu-ccd-query-import/v0.1/fixtures/armadillo-rollers-3ee-mma-bool.json"
        );
        let mut queries = parse_queries_csv(CSV, QueryKind::EdgeEdge).unwrap();
        attach_ground_truth(&mut queries, &parse_boolean_json(BOOLS).unwrap()).unwrap();

        let row0 = [
            "nyu-fixture-v1",
            "armadillo-rollers",
            "3",
            "edge_edge",
            "0",
            "forwarded",
            "",
            "",
            "",
            "",
            "",
            "no_collision",
        ]
        .join("\t");
        let row1 = [
            "nyu-fixture-v1",
            "armadillo-rollers",
            "3",
            "edge_edge",
            "1",
            "fallback",
            "unsupported_motion",
            "",
            "",
            "",
            "",
            "no_collision",
        ]
        .join("\t");
        let results = parse_query_run_results_tsv(&[row0, row1].join("\n")).unwrap();
        let profile = RunProfile::new(
            RunProvenance::new(
                "symtropy-adapter-smoke",
                "v0.1",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            ),
            MotionProfile::LinearizedConstantVelocity,
            ArithmeticProfile::Binary64,
        );
        let score = evaluate_imported_queries_with_profile(
            profile,
            "nyu-fixture-v1",
            "armadillo-rollers",
            3,
            &queries,
            &results,
        )
        .unwrap();

        assert_eq!(score.counts.total_queries, 2);
        assert_eq!(score.counts.oracle_negative_queries, 2);
        assert_eq!(score.counts.fallback_queries, 1);
        assert_eq!(score.counts.candidate_false_positives, 2);
        assert!(!score.benchmark_qualified());
    }

    #[test]
    fn rejects_missing_oracle_ground_truth() {
        let csv = (0..8)
            .map(|index| format!("{index},1,0,1,0,1"))
            .collect::<Vec<_>>()
            .join("\n");
        let queries = parse_queries_csv(&csv, QueryKind::EdgeEdge).unwrap();
        assert_eq!(
            QueryEvaluation::from_imported(
                "dataset",
                "scene",
                0,
                &queries[0],
                CandidateDisposition::Forwarded,
                NarrowphaseOutcome::NoCollision,
            ),
            Err(EvaluationError::MissingOracleGroundTruth)
        );
    }
}
