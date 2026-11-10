//! Evaluator-side memory-operation fidelity contract.
//!
//! Measures Storage, Update and Compression as separate operations. Correctness
//! is derived by comparing evaluator-expected and independently observed effect
//! digests; the policy cannot self-report a pass. Missing receipts, missing
//! readback evidence and invalid runs remain unresolved in the applicable
//! denominator. This is a synthetic evidence contract, not a memory store,
//! native MEMOBench adapter, or task-success metric.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryOperationKind {
    Storage,
    Update,
    Compression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOperationApplicability {
    Applicable,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOperationOutcome {
    Correct,
    Incorrect,
    Unresolved,
    NotApplicable,
}

/// Evaluator-owned expected case. `expected_effect_sha256` is the canonical
/// evaluator-defined semantic readback digest, not necessarily a byte-for-byte
/// memory representation. For Compression it represents required-fact/query
/// preservation, so a different compressed representation may still be correct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedMemoryOperation {
    pub case_id: String,
    pub task_id: String,
    pub episode_id: String,
    /// Independent sampling unit, e.g. a held-out scene/seed cluster.
    /// Multiple operation cases may share the same unit.
    pub independence_unit_id: String,
    pub sequence: u64,
    pub operation: MemoryOperationKind,
    pub applicability: MemoryOperationApplicability,
    pub source_event_sha256: String,
    /// Exact evaluator query whose semantic answer is being measured.
    pub query_sha256: String,
    pub expected_effect_sha256: Option<String>,
}

/// Frozen identity of one operation-level memory evaluation cohort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryOperationManifest {
    pub protocol_sha256: String,
    pub operation_roster_sha256: String,
    pub policy_artifact_sha256: String,
    pub memory_system_sha256: String,
    pub evaluator_sha256: String,
    pub query_roster_sha256: String,
    /// Frozen task/scene/seed/split, embodiment, action executor and resource-budget identity.
    pub control_manifest_sha256: String,
    pub expected_case_count: usize,
    /// Complete task roster for this operation-level evaluation cohort.
    pub expected_task_ids: Vec<String>,
    /// Cases may interleave episodes, but sequences must strictly increase within each episode.
    pub cases: Vec<ExpectedMemoryOperation>,
}

/// Independent evaluator/readback receipt for one expected operation case.
/// The receipt contains measurements and effect identities, not a self-reported
/// Correct/Incorrect label. The auditor derives that outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryOperationReceipt {
    pub case_id: String,
    pub task_id: String,
    pub episode_id: String,
    pub independence_unit_id: String,
    pub sequence: u64,
    pub operation: MemoryOperationKind,
    /// Exact cohort context that produced this receipt.
    pub protocol_sha256: String,
    pub operation_roster_sha256: String,
    pub policy_artifact_sha256: String,
    pub memory_system_sha256: String,
    pub evaluator_sha256: String,
    pub query_roster_sha256: String,
    pub control_manifest_sha256: String,
    pub observed_source_event_sha256: Option<String>,
    pub observed_query_sha256: Option<String>,
    /// Evaluator-derived semantic readback digest; for Compression this is
    /// required-fact preservation, not the compressed snapshot's byte digest.
    pub observed_effect_sha256: Option<String>,
    pub memory_snapshot_before_sha256: Option<String>,
    pub memory_snapshot_after_sha256: Option<String>,
    pub evaluator_receipt_sha256: Option<String>,
    pub memory_bytes_before: Option<u64>,
    pub memory_bytes_after: Option<u64>,
    pub latency_ms: Option<u64>,
    /// False for timeout, invalid run, interruption or lost evaluator result.
    pub valid_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOperationError {
    EmptyManifest,
    EmptyIdentity,
    InvalidDigest,
    ExpectedCaseCountMismatch,
    EmptyTaskRoster,
    DuplicateTask,
    TaskSetMismatch,
    UnexpectedTask,
    DuplicateCase,
    DuplicateSequence,
    NonMonotonicSequence,
    EpisodeIndependenceUnitMismatch,
    ApplicableCaseMissingExpectedEffect,
    NotApplicableCaseHasExpectedEffect,
    DuplicateReceipt,
    UnknownReceipt,
    ReceiptForNotApplicableCase,
    ReceiptIdentityMismatch,
    ReceiptContextMismatch,
    ComparisonProtocolMismatch,
    ComparisonOperationRosterMismatch,
    ComparisonQueryRosterMismatch,
    ComparisonControlManifestMismatch,
    ComparisonPolicyArtifactMismatch,
    ComparisonEvaluatorMismatch,
    ComparisonExpectedCountMismatch,
    ComparisonTaskRosterMismatch,
    ComparisonApplicabilityMismatch,
    ComparisonUnitRosterMismatch,
    BootstrapInvalidConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryOperationCaseResult {
    pub case_id: String,
    pub task_id: String,
    pub episode_id: String,
    pub independence_unit_id: String,
    pub operation: MemoryOperationKind,
    pub source_event_sha256: String,
    pub query_sha256: String,
    pub expected_effect_sha256: Option<String>,
    pub observed_query_sha256: Option<String>,
    pub observed_source_event_sha256: Option<String>,
    pub observed_effect_sha256: Option<String>,
    pub memory_snapshot_before_sha256: Option<String>,
    pub memory_snapshot_after_sha256: Option<String>,
    pub evaluator_receipt_sha256: Option<String>,
    pub valid_run: Option<bool>,
    pub outcome: MemoryOperationOutcome,
    pub memory_bytes_before: Option<u64>,
    pub memory_bytes_after: Option<u64>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperationSummary {
    /// None for the operation-wide aggregate; Some for task-stratified rows.
    pub task_id: Option<String>,
    pub operation: MemoryOperationKind,
    pub applicable_count: usize,
    pub not_applicable_count: usize,
    pub correct_count: usize,
    pub incorrect_count: usize,
    pub unresolved_count: usize,
    /// Correct / all applicable cases; unresolved cases are not dropped.
    pub fidelity_rate: Option<f64>,
    /// (Correct + Incorrect) / applicable cases.
    pub evidence_coverage: Option<f64>,
    /// Total measured bytes after / total measured bytes before.
    /// Only complete compression measurements contribute.
    pub compression_ratio: Option<f64>,
    pub compression_measurement_count: usize,
    /// Distinct preregistered scene/seed independence units represented here.
    pub independent_unit_count: usize,
    pub mean_latency_ms: Option<f64>,
    pub latency_measurement_count: usize,
    /// Equal-task macro means over task rows with at least one applicable case.
    /// These are populated on operation-wide rows, not task rows.
    pub applicable_task_count: usize,
    pub task_macro_fidelity_rate: Option<f64>,
    pub task_macro_evidence_coverage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperationUnitSummary {
    pub task_id: String,
    pub independence_unit_id: String,
    pub operation: MemoryOperationKind,
    pub applicable_count: usize,
    pub not_applicable_count: usize,
    pub correct_count: usize,
    pub incorrect_count: usize,
    pub unresolved_count: usize,
    pub fidelity_rate: Option<f64>,
    pub evidence_coverage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperationPairedUnitDelta {
    pub task_id: String,
    pub independence_unit_id: String,
    pub operation: MemoryOperationKind,
    pub baseline_fidelity_rate: Option<f64>,
    pub candidate_fidelity_rate: Option<f64>,
    pub fidelity_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperationPairedSummary {
    pub operation: MemoryOperationKind,
    pub paired_unit_count: usize,
    pub contributing_unit_count: usize,
    pub baseline_mean_unit_fidelity: Option<f64>,
    pub candidate_mean_unit_fidelity: Option<f64>,
    pub mean_paired_fidelity_delta: Option<f64>,
    pub candidate_wins: usize,
    pub baseline_wins: usize,
    pub ties: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperationPairedComparison {
    pub protocol_sha256: String,
    pub operation_roster_sha256: String,
    pub query_roster_sha256: String,
    pub control_manifest_sha256: String,
    pub policy_artifact_sha256: String,
    pub evaluator_sha256: String,
    pub baseline_memory_system_sha256: String,
    pub candidate_memory_system_sha256: String,
    pub paired_unit_count: usize,
    pub unit_deltas: Vec<MemoryOperationPairedUnitDelta>,
    pub summaries: Vec<MemoryOperationPairedSummary>,
}

/// Deterministic task-stratified cluster-bootstrap configuration.
/// Each task is weighted equally; independent scene/seed units are resampled
/// with replacement within each task. Checkpoints are never resampled alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusterBootstrapConfig {
    pub replicates: usize,
    pub seed: u64,
    pub confidence_level: f64,
    pub minimum_independent_units_per_task: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterBootstrapStatus {
    SufficientIndependentUnits,
    Underpowered,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterBootstrapOperationSummary {
    pub operation: MemoryOperationKind,
    pub status: ClusterBootstrapStatus,
    pub contributing_task_count: usize,
    pub independent_unit_count: usize,
    pub estimate_task_macro_mean_delta: Option<f64>,
    pub lower_percentile_bound: Option<f64>,
    pub upper_percentile_bound: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperationBootstrapReport {
    pub seed: u64,
    pub replicates: usize,
    pub confidence_level: f64,
    pub minimum_independent_units_per_task: usize,
    pub summaries: Vec<ClusterBootstrapOperationSummary>,
}

/// Small fixed PRNG keeps the bootstrap deterministic without adding a crate
/// dependency or coupling results to a third-party RNG implementation.
#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn index_below(&mut self, upper_bound: usize) -> usize {
        let bound = upper_bound as u64;
        let rejection_threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= rejection_threshold {
                return (value % bound) as usize;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperationAudit {
    pub protocol_sha256: String,
    pub operation_roster_sha256: String,
    pub policy_artifact_sha256: String,
    pub memory_system_sha256: String,
    pub evaluator_sha256: String,
    pub query_roster_sha256: String,
    pub control_manifest_sha256: String,
    pub expected_case_count: usize,
    pub receipt_count: usize,
    pub results: Vec<MemoryOperationCaseResult>,
    /// Operation-wide micro summaries.
    pub summaries: Vec<MemoryOperationSummary>,
    /// Task-stratified summaries to expose easy-task / hard-task masking.
    pub task_summaries: Vec<MemoryOperationSummary>,
    /// Independent scene/seed-unit rows for cluster-aware resampling.
    pub unit_summaries: Vec<MemoryOperationUnitSummary>,
}

#[derive(Debug, Default)]
struct UnitAccumulator {
    applicable_count: usize,
    not_applicable_count: usize,
    correct_count: usize,
    incorrect_count: usize,
    unresolved_count: usize,
}

impl UnitAccumulator {
    fn observe(&mut self, outcome: MemoryOperationOutcome) {
        match outcome {
            MemoryOperationOutcome::NotApplicable => self.not_applicable_count += 1,
            MemoryOperationOutcome::Correct => {
                self.applicable_count += 1;
                self.correct_count += 1;
            }
            MemoryOperationOutcome::Incorrect => {
                self.applicable_count += 1;
                self.incorrect_count += 1;
            }
            MemoryOperationOutcome::Unresolved => {
                self.applicable_count += 1;
                self.unresolved_count += 1;
            }
        }
    }

    fn finish(
        self,
        task_id: String,
        independence_unit_id: String,
        operation: MemoryOperationKind,
    ) -> MemoryOperationUnitSummary {
        MemoryOperationUnitSummary {
            task_id,
            independence_unit_id,
            operation,
            applicable_count: self.applicable_count,
            not_applicable_count: self.not_applicable_count,
            correct_count: self.correct_count,
            incorrect_count: self.incorrect_count,
            unresolved_count: self.unresolved_count,
            fidelity_rate: (self.applicable_count > 0).then(|| {
                self.correct_count as f64 / self.applicable_count as f64
            }),
            evidence_coverage: (self.applicable_count > 0).then(|| {
                (self.correct_count + self.incorrect_count) as f64
                    / self.applicable_count as f64
            }),
        }
    }
}

#[derive(Debug, Default)]
struct OperationAccumulator {
    applicable_count: usize,
    not_applicable_count: usize,
    correct_count: usize,
    incorrect_count: usize,
    unresolved_count: usize,
    compression_measurement_count: usize,
    compression_bytes_before: u128,
    compression_bytes_after: u128,
    latency_measurement_count: usize,
    latency_sum_ms: u128,
    independent_units: BTreeSet<(String, String)>,
}

impl OperationAccumulator {
    fn observe(&mut self, result: &MemoryOperationCaseResult) {
        match result.outcome {
            MemoryOperationOutcome::NotApplicable => {
                self.not_applicable_count += 1;
                return;
            }
            MemoryOperationOutcome::Correct => {
                self.applicable_count += 1;
                self.correct_count += 1;
            }
            MemoryOperationOutcome::Incorrect => {
                self.applicable_count += 1;
                self.incorrect_count += 1;
            }
            MemoryOperationOutcome::Unresolved => {
                self.applicable_count += 1;
                self.unresolved_count += 1;
            }
        }
        self.independent_units.insert((result.task_id.clone(), result.independence_unit_id.clone()));

        if result.operation == MemoryOperationKind::Compression {
            if let (Some(before), Some(after)) =
                (result.memory_bytes_before, result.memory_bytes_after)
            {
                if before > 0 {
                    self.compression_measurement_count += 1;
                    self.compression_bytes_before += u128::from(before);
                    self.compression_bytes_after += u128::from(after);
                }
            }
        }

        if let Some(latency) = result.latency_ms {
            self.latency_measurement_count += 1;
            self.latency_sum_ms += u128::from(latency);
        }
    }

    fn finish(
        self,
        operation: MemoryOperationKind,
        task_id: Option<String>,
    ) -> MemoryOperationSummary {
        let fidelity_rate = (self.applicable_count > 0).then(|| {
            self.correct_count as f64 / self.applicable_count as f64
        });
        let evidence_coverage = (self.applicable_count > 0).then(|| {
            (self.correct_count + self.incorrect_count) as f64
                / self.applicable_count as f64
        });
        let compression_ratio = (self.compression_measurement_count > 0
            && self.compression_bytes_before > 0)
            .then(|| {
                self.compression_bytes_after as f64 / self.compression_bytes_before as f64
            });
        let mean_latency_ms = (self.latency_measurement_count > 0).then(|| {
            self.latency_sum_ms as f64 / self.latency_measurement_count as f64
        });

        MemoryOperationSummary {
            task_id,
            operation,
            applicable_count: self.applicable_count,
            not_applicable_count: self.not_applicable_count,
            correct_count: self.correct_count,
            incorrect_count: self.incorrect_count,
            unresolved_count: self.unresolved_count,
            fidelity_rate,
            evidence_coverage,
            compression_ratio,
            compression_measurement_count: self.compression_measurement_count,
            independent_unit_count: self.independent_units.len(),
            mean_latency_ms,
            latency_measurement_count: self.latency_measurement_count,
            applicable_task_count: 0,
            task_macro_fidelity_rate: None,
            task_macro_evidence_coverage: None,
        }
    }
}

/// Audit a frozen operation manifest against independent evaluator receipts.
///
/// A missing receipt, invalid run, missing readback, missing snapshot lineage or
/// missing evaluator receipt is unresolved, not an implicit success/failure.
/// The operation result is derived by comparing source lineage and observed
/// effect digests with the evaluator-owned manifest. This function cannot prove
/// that the supplied digests or evaluator receipts are truthful/authentic.
pub fn audit_memory_operations(
    manifest: &MemoryOperationManifest,
    receipts: &[MemoryOperationReceipt],
) -> Result<MemoryOperationAudit, MemoryOperationError> {
    validate_manifest(manifest)?;

    let mut expected_by_id = BTreeMap::new();
    for case in &manifest.cases {
        expected_by_id.insert(case.case_id.as_str(), case);
    }

    let mut receipt_by_id = BTreeMap::new();
    for receipt in receipts {
        validate_receipt_digests(receipt)?;
        if !expected_by_id.contains_key(receipt.case_id.as_str()) {
            return Err(MemoryOperationError::UnknownReceipt);
        }
        if receipt_by_id.insert(receipt.case_id.as_str(), receipt).is_some() {
            return Err(MemoryOperationError::DuplicateReceipt);
        }
    }

    let mut results = Vec::with_capacity(manifest.cases.len());
    let mut accumulators = BTreeMap::<MemoryOperationKind, OperationAccumulator>::new();
    let mut task_accumulators =
        BTreeMap::<(MemoryOperationKind, String), OperationAccumulator>::new();
    let mut unit_accumulators =
        BTreeMap::<(String, String, MemoryOperationKind), UnitAccumulator>::new();

    for case in &manifest.cases {
        let outcome = match case.applicability {
            MemoryOperationApplicability::NotApplicable => {
                if receipt_by_id.contains_key(case.case_id.as_str()) {
                    return Err(MemoryOperationError::ReceiptForNotApplicableCase);
                }
                MemoryOperationOutcome::NotApplicable
            }
            MemoryOperationApplicability::Applicable => {
                match receipt_by_id.get(case.case_id.as_str()) {
                    None => MemoryOperationOutcome::Unresolved,
                    Some(receipt) => {
                        validate_receipt_identity(manifest, case, receipt)?;
                        derive_outcome(case, receipt)
                    }
                }
            }
        };

        let receipt = receipt_by_id.get(case.case_id.as_str()).copied();
        let result = MemoryOperationCaseResult {
            case_id: case.case_id.clone(),
            task_id: case.task_id.clone(),
            episode_id: case.episode_id.clone(),
            independence_unit_id: case.independence_unit_id.clone(),
            operation: case.operation,
            source_event_sha256: case.source_event_sha256.clone(),
            query_sha256: case.query_sha256.clone(),
            expected_effect_sha256: case.expected_effect_sha256.clone(),
            observed_query_sha256: receipt.and_then(|row| row.observed_query_sha256.clone()),
            observed_source_event_sha256: receipt
                .and_then(|row| row.observed_source_event_sha256.clone()),
            observed_effect_sha256: receipt.and_then(|row| row.observed_effect_sha256.clone()),
            memory_snapshot_before_sha256: receipt
                .and_then(|row| row.memory_snapshot_before_sha256.clone()),
            memory_snapshot_after_sha256: receipt
                .and_then(|row| row.memory_snapshot_after_sha256.clone()),
            evaluator_receipt_sha256: receipt.and_then(|row| row.evaluator_receipt_sha256.clone()),
            valid_run: receipt.map(|row| row.valid_run),
            outcome,
            memory_bytes_before: receipt.and_then(|row| row.memory_bytes_before),
            memory_bytes_after: receipt.and_then(|row| row.memory_bytes_after),
            latency_ms: receipt.and_then(|row| row.latency_ms),
        };

        accumulators
            .entry(case.operation)
            .or_default()
            .observe(&result);
        task_accumulators
            .entry((case.operation, case.task_id.clone()))
            .or_default()
            .observe(&result);
        unit_accumulators
            .entry((
                case.task_id.clone(),
                case.independence_unit_id.clone(),
                case.operation,
            ))
            .or_default()
            .observe(outcome);
        results.push(result);
    }

    let task_summaries = task_accumulators
        .into_iter()
        .map(|((operation, task_id), accumulator)| accumulator.finish(operation, Some(task_id)))
        .collect::<Vec<_>>();
    let unit_summaries = unit_accumulators
        .into_iter()
        .map(|((task_id, independence_unit_id, operation), accumulator)| {
            accumulator.finish(task_id, independence_unit_id, operation)
        })
        .collect::<Vec<_>>();

    let mut summaries = [
        MemoryOperationKind::Storage,
        MemoryOperationKind::Update,
        MemoryOperationKind::Compression,
    ]
    .into_iter()
    .filter_map(|operation| {
        accumulators
            .remove(&operation)
            .map(|accumulator| accumulator.finish(operation, None))
    })
    .collect::<Vec<_>>();

    for summary in &mut summaries {
        let rows = task_summaries
            .iter()
            .filter(|row| row.operation == summary.operation && row.applicable_count > 0)
            .collect::<Vec<_>>();
        summary.applicable_task_count = rows.len();
        if !rows.is_empty() {
            summary.task_macro_fidelity_rate = Some(
                rows.iter()
                    .filter_map(|row| row.fidelity_rate)
                    .sum::<f64>() / rows.len() as f64,
            );
            summary.task_macro_evidence_coverage = Some(
                rows.iter()
                    .filter_map(|row| row.evidence_coverage)
                    .sum::<f64>() / rows.len() as f64,
            );
        }
    }
    Ok(MemoryOperationAudit {
        protocol_sha256: manifest.protocol_sha256.clone(),
        operation_roster_sha256: manifest.operation_roster_sha256.clone(),
        policy_artifact_sha256: manifest.policy_artifact_sha256.clone(),
        memory_system_sha256: manifest.memory_system_sha256.clone(),
        evaluator_sha256: manifest.evaluator_sha256.clone(),
        query_roster_sha256: manifest.query_roster_sha256.clone(),
        control_manifest_sha256: manifest.control_manifest_sha256.clone(),
        expected_case_count: manifest.expected_case_count,
        receipt_count: receipts.len(),
        results,
        summaries,
        task_summaries,
        unit_summaries,
    })
}

/// Compare two memory-system arms on the exact same evaluator/query/task/unit
/// roster and frozen control bundle. The policy artifact is held constant so
/// this is a memory-only paired ablation. It reports descriptive per-unit
/// deltas; it does not compute confidence intervals or p-values. Resample the
/// returned independent-unit rows (not individual checkpoints) for inference.
pub fn compare_memory_operation_arms(
    baseline: &MemoryOperationAudit,
    candidate: &MemoryOperationAudit,
) -> Result<MemoryOperationPairedComparison, MemoryOperationError> {
    if baseline.protocol_sha256 != candidate.protocol_sha256 {
        return Err(MemoryOperationError::ComparisonProtocolMismatch);
    }
    if baseline.operation_roster_sha256 != candidate.operation_roster_sha256 {
        return Err(MemoryOperationError::ComparisonOperationRosterMismatch);
    }
    if baseline.query_roster_sha256 != candidate.query_roster_sha256 {
        return Err(MemoryOperationError::ComparisonQueryRosterMismatch);
    }
    if baseline.control_manifest_sha256 != candidate.control_manifest_sha256 {
        return Err(MemoryOperationError::ComparisonControlManifestMismatch);
    }
    if baseline.policy_artifact_sha256 != candidate.policy_artifact_sha256 {
        return Err(MemoryOperationError::ComparisonPolicyArtifactMismatch);
    }
    if baseline.evaluator_sha256 != candidate.evaluator_sha256 {
        return Err(MemoryOperationError::ComparisonEvaluatorMismatch);
    }
    if baseline.expected_case_count != candidate.expected_case_count {
        return Err(MemoryOperationError::ComparisonExpectedCountMismatch);
    }
    if baseline.unit_summaries.is_empty()
        || candidate.unit_summaries.is_empty()
        || baseline.unit_summaries.len() != candidate.unit_summaries.len()
    {
        return Err(MemoryOperationError::ComparisonUnitRosterMismatch);
    }

    let baseline_task_ids = baseline.unit_summaries.iter()
        .map(|unit| unit.task_id.as_str())
        .collect::<BTreeSet<_>>();
    let candidate_task_ids = candidate.unit_summaries.iter()
        .map(|unit| unit.task_id.as_str())
        .collect::<BTreeSet<_>>();
    if baseline_task_ids != candidate_task_ids {
        return Err(MemoryOperationError::ComparisonTaskRosterMismatch);
    }

    let mut baseline_units = BTreeMap::new();
    for unit in &baseline.unit_summaries {
        let key = (
            unit.task_id.as_str(),
            unit.independence_unit_id.as_str(),
            unit.operation,
        );
        if baseline_units.insert(key, unit).is_some() {
            return Err(MemoryOperationError::ComparisonUnitRosterMismatch);
        }
    }
    let mut candidate_units = BTreeMap::new();
    for unit in &candidate.unit_summaries {
        let key = (
            unit.task_id.as_str(),
            unit.independence_unit_id.as_str(),
            unit.operation,
        );
        if candidate_units.insert(key, unit).is_some() {
            return Err(MemoryOperationError::ComparisonUnitRosterMismatch);
        }
    }
    if baseline_units.keys().copied().collect::<BTreeSet<_>>()
        != candidate_units.keys().copied().collect::<BTreeSet<_>>()
    {
        return Err(MemoryOperationError::ComparisonUnitRosterMismatch);
    }

    let mut unit_deltas = Vec::with_capacity(baseline_units.len());
    for (key, baseline_unit) in &baseline_units {
        let candidate_unit = candidate_units.get(key)
            .ok_or(MemoryOperationError::ComparisonUnitRosterMismatch)?;
        if baseline_unit.applicable_count != candidate_unit.applicable_count
            || baseline_unit.not_applicable_count != candidate_unit.not_applicable_count
        {
            return Err(MemoryOperationError::ComparisonApplicabilityMismatch);
        }
        let baseline_rate = baseline_unit.fidelity_rate;
        let candidate_rate = candidate_unit.fidelity_rate;
        let delta = match (baseline_rate, candidate_rate) {
            (Some(base), Some(candidate)) => Some(candidate - base),
            (None, None) => None,
            _ => return Err(MemoryOperationError::ComparisonApplicabilityMismatch),
        };
        unit_deltas.push(MemoryOperationPairedUnitDelta {
            task_id: baseline_unit.task_id.clone(),
            independence_unit_id: baseline_unit.independence_unit_id.clone(),
            operation: baseline_unit.operation,
            baseline_fidelity_rate: baseline_rate,
            candidate_fidelity_rate: candidate_rate,
            fidelity_delta: delta,
        });
    }

    let mut summaries = Vec::new();
    for operation in [
        MemoryOperationKind::Storage,
        MemoryOperationKind::Update,
        MemoryOperationKind::Compression,
    ] {
        let rows = unit_deltas.iter()
            .filter(|row| row.operation == operation)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        let contributing = rows.iter()
            .filter(|row| row.fidelity_delta.is_some())
            .collect::<Vec<_>>();
        let mut candidate_wins = 0;
        let mut baseline_wins = 0;
        let mut ties = 0;
        for row in &contributing {
            if let Some(delta) = row.fidelity_delta {
                match delta.total_cmp(&0.0) {
                    std::cmp::Ordering::Greater => candidate_wins += 1,
                    std::cmp::Ordering::Less => baseline_wins += 1,
                    std::cmp::Ordering::Equal => ties += 1,
                }
            }
        }
        let mean = |values: Vec<f64>| {
            (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
        };
        summaries.push(MemoryOperationPairedSummary {
            operation,
            paired_unit_count: rows.len(),
            contributing_unit_count: contributing.len(),
            baseline_mean_unit_fidelity: mean(contributing.iter()
                .filter_map(|row| row.baseline_fidelity_rate).collect()),
            candidate_mean_unit_fidelity: mean(contributing.iter()
                .filter_map(|row| row.candidate_fidelity_rate).collect()),
            mean_paired_fidelity_delta: mean(contributing.iter()
                .filter_map(|row| row.fidelity_delta).collect()),
            candidate_wins,
            baseline_wins,
            ties,
        });
    }

    Ok(MemoryOperationPairedComparison {
        protocol_sha256: baseline.protocol_sha256.clone(),
        operation_roster_sha256: baseline.operation_roster_sha256.clone(),
        query_roster_sha256: baseline.query_roster_sha256.clone(),
        control_manifest_sha256: baseline.control_manifest_sha256.clone(),
        policy_artifact_sha256: baseline.policy_artifact_sha256.clone(),
        evaluator_sha256: baseline.evaluator_sha256.clone(),
        baseline_memory_system_sha256: baseline.memory_system_sha256.clone(),
        candidate_memory_system_sha256: candidate.memory_system_sha256.clone(),
        paired_unit_count: unit_deltas.len(),
        unit_deltas,
        summaries,
    })
}

/// Produce percentile intervals from paired deltas using a task-stratified
/// cluster bootstrap. Tasks are equally weighted; within each task, independent
/// scene/seed units are sampled with replacement. This is descriptive inferential
/// support, not a safety guarantee. The caller must preregister the confidence
/// level, seed, replicate count and minimum unit rule before opening held-out data.
pub fn bootstrap_memory_operation_comparison(
    comparison: &MemoryOperationPairedComparison,
    config: ClusterBootstrapConfig,
) -> Result<MemoryOperationBootstrapReport, MemoryOperationError> {
    if config.replicates == 0
        || config.replicates > 1_000_000
        || !config.confidence_level.is_finite()
        || config.confidence_level <= 0.5
        || config.confidence_level >= 1.0
        || config.minimum_independent_units_per_task < 2
    {
        return Err(MemoryOperationError::BootstrapInvalidConfig);
    }

    let mut grouped = BTreeMap::<
        (MemoryOperationKind, String),
        Vec<f64>,
    >::new();
    for row in &comparison.unit_deltas {
        if let Some(delta) = row.fidelity_delta {
            grouped
                .entry((row.operation, row.task_id.clone()))
                .or_default()
                .push(delta);
        }
    }

    let mut summaries = Vec::with_capacity(comparison.summaries.len());
    for (operation_index, paired_summary) in comparison.summaries.iter().enumerate() {
        let task_groups = grouped
            .iter()
            .filter(|((operation, _), _)| *operation == paired_summary.operation)
            .map(|((_, task_id), values)| (task_id, values))
            .collect::<Vec<_>>();
        let independent_unit_count = task_groups.iter().map(|(_, rows)| rows.len()).sum();
        let contributing_task_count = task_groups.len();
        let task_means = task_groups
            .iter()
            .map(|(_, values)| values.iter().sum::<f64>() / values.len() as f64)
            .collect::<Vec<_>>();
        let estimate = (!task_means.is_empty())
            .then(|| task_means.iter().sum::<f64>() / task_means.len() as f64);
        let sufficient = !task_groups.is_empty()
            && task_groups.iter().all(|(_, rows)| {
                rows.len() >= config.minimum_independent_units_per_task
            });

        if !sufficient {
            summaries.push(ClusterBootstrapOperationSummary {
                operation: paired_summary.operation,
                status: ClusterBootstrapStatus::Underpowered,
                contributing_task_count,
                independent_unit_count,
                estimate_task_macro_mean_delta: estimate,
                lower_percentile_bound: None,
                upper_percentile_bound: None,
            });
            continue;
        }

        let mut rng = SplitMix64::new(
            config.seed ^ ((operation_index as u64 + 1).wrapping_mul(0xD6E8_FEB8_6659_FD93)),
        );
        let mut bootstrap_estimates = Vec::with_capacity(config.replicates);
        for _ in 0..config.replicates {
            let mut resampled_task_means = Vec::with_capacity(task_groups.len());
            for (_, unit_deltas) in &task_groups {
                let mut sum = 0.0;
                for _ in 0..unit_deltas.len() {
                    let index = rng.index_below(unit_deltas.len());
                    sum += unit_deltas[index];
                }
                resampled_task_means.push(sum / unit_deltas.len() as f64);
            }
            bootstrap_estimates.push(
                resampled_task_means.iter().sum::<f64>() / resampled_task_means.len() as f64,
            );
        }
        bootstrap_estimates.sort_by(f64::total_cmp);
        let tail_probability = (1.0 - config.confidence_level) / 2.0;
        let lower_percentile_bound = percentile(&bootstrap_estimates, tail_probability);
        let upper_percentile_bound = percentile(&bootstrap_estimates, 1.0 - tail_probability);

        summaries.push(ClusterBootstrapOperationSummary {
            operation: paired_summary.operation,
            status: ClusterBootstrapStatus::SufficientIndependentUnits,
            contributing_task_count,
            independent_unit_count,
            estimate_task_macro_mean_delta: estimate,
            lower_percentile_bound,
            upper_percentile_bound,
        });
    }

    Ok(MemoryOperationBootstrapReport {
        seed: config.seed,
        replicates: config.replicates,
        confidence_level: config.confidence_level,
        minimum_independent_units_per_task: config.minimum_independent_units_per_task,
        summaries,
    })
}

fn percentile(sorted_values: &[f64], probability: f64) -> Option<f64> {
    if sorted_values.is_empty() || !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return None;
    }
    let position = probability * (sorted_values.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    let weight = position - lower_index as f64;
    Some(
        sorted_values[lower_index] * (1.0 - weight)
            + sorted_values[upper_index] * weight,
    )
}

fn validate_manifest(
    manifest: &MemoryOperationManifest,
) -> Result<(), MemoryOperationError> {
    if manifest.cases.is_empty() {
        return Err(MemoryOperationError::EmptyManifest);
    }
    if manifest.expected_case_count != manifest.cases.len() {
        return Err(MemoryOperationError::ExpectedCaseCountMismatch);
    }
    for digest in [
        manifest.protocol_sha256.as_str(),
        manifest.operation_roster_sha256.as_str(),
        manifest.policy_artifact_sha256.as_str(),
        manifest.memory_system_sha256.as_str(),
        manifest.evaluator_sha256.as_str(),
        manifest.query_roster_sha256.as_str(),
        manifest.control_manifest_sha256.as_str(),
    ] {
        if !is_sha256_hex(digest) {
            return Err(MemoryOperationError::InvalidDigest);
        }
    }

    if manifest.expected_task_ids.is_empty() {
        return Err(MemoryOperationError::EmptyTaskRoster);
    }
    let mut expected_tasks = BTreeSet::new();
    for task_id in &manifest.expected_task_ids {
        if task_id.trim().is_empty() {
            return Err(MemoryOperationError::EmptyIdentity);
        }
        if !expected_tasks.insert(task_id.as_str()) {
            return Err(MemoryOperationError::DuplicateTask);
        }
    }

    let mut observed_tasks = BTreeSet::new();
    let mut seen_cases = BTreeSet::new();
    let mut last_sequence = BTreeMap::<(&str, &str), u64>::new();
    let mut seen_sequences = BTreeMap::<(&str, &str), BTreeSet<u64>>::new();
    let mut episode_units = BTreeMap::<(&str, &str), &str>::new();

    for case in &manifest.cases {
        if case.case_id.trim().is_empty()
            || case.task_id.trim().is_empty()
            || case.episode_id.trim().is_empty()
            || case.independence_unit_id.trim().is_empty()
        {
            return Err(MemoryOperationError::EmptyIdentity);
        }
        if !expected_tasks.contains(case.task_id.as_str()) {
            return Err(MemoryOperationError::UnexpectedTask);
        }
        observed_tasks.insert(case.task_id.as_str());
        let episode_key = (case.task_id.as_str(), case.episode_id.as_str());
        if episode_units.insert(episode_key, case.independence_unit_id.as_str())
            .is_some_and(|unit| unit != case.independence_unit_id.as_str())
        {
            return Err(MemoryOperationError::EpisodeIndependenceUnitMismatch);
        }
        if !seen_cases.insert(case.case_id.as_str()) {
            return Err(MemoryOperationError::DuplicateCase);
        }
        if !is_sha256_hex(&case.source_event_sha256)
            || !is_sha256_hex(&case.query_sha256)
        {
            return Err(MemoryOperationError::InvalidDigest);
        }

        match (case.applicability, case.expected_effect_sha256.as_deref()) {
            (MemoryOperationApplicability::Applicable, None) => {
                return Err(MemoryOperationError::ApplicableCaseMissingExpectedEffect);
            }
            (MemoryOperationApplicability::NotApplicable, Some(_)) => {
                return Err(MemoryOperationError::NotApplicableCaseHasExpectedEffect);
            }
            (_, Some(digest)) if !is_sha256_hex(digest) => {
                return Err(MemoryOperationError::InvalidDigest);
            }
            _ => {}
        }

        let episode_key = (case.task_id.as_str(), case.episode_id.as_str());
        let sequences = seen_sequences.entry(episode_key).or_default();
        if !sequences.insert(case.sequence) {
            return Err(MemoryOperationError::DuplicateSequence);
        }
        if let Some(previous) = last_sequence.get(&episode_key) {
            if case.sequence <= *previous {
                return Err(MemoryOperationError::NonMonotonicSequence);
            }
        }
        last_sequence.insert(episode_key, case.sequence);
    }
    if observed_tasks != expected_tasks {
        return Err(MemoryOperationError::TaskSetMismatch);
    }
    Ok(())
}

fn validate_receipt_digests(
    receipt: &MemoryOperationReceipt,
) -> Result<(), MemoryOperationError> {
    if receipt.case_id.trim().is_empty()
        || receipt.task_id.trim().is_empty()
        || receipt.episode_id.trim().is_empty()
        || receipt.independence_unit_id.trim().is_empty()
    {
        return Err(MemoryOperationError::EmptyIdentity);
    }

    for digest in [
        Some(receipt.protocol_sha256.as_str()),
        Some(receipt.operation_roster_sha256.as_str()),
        Some(receipt.policy_artifact_sha256.as_str()),
        Some(receipt.memory_system_sha256.as_str()),
        Some(receipt.evaluator_sha256.as_str()),
        Some(receipt.query_roster_sha256.as_str()),
        Some(receipt.control_manifest_sha256.as_str()),
        receipt.observed_source_event_sha256.as_deref(),
        receipt.observed_query_sha256.as_deref(),
        receipt.observed_effect_sha256.as_deref(),
        receipt.memory_snapshot_before_sha256.as_deref(),
        receipt.memory_snapshot_after_sha256.as_deref(),
        receipt.evaluator_receipt_sha256.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !is_sha256_hex(digest) {
            return Err(MemoryOperationError::InvalidDigest);
        }
    }
    Ok(())
}

fn validate_receipt_identity(
    manifest: &MemoryOperationManifest,
    case: &ExpectedMemoryOperation,
    receipt: &MemoryOperationReceipt,
) -> Result<(), MemoryOperationError> {
    if receipt.protocol_sha256 != manifest.protocol_sha256
        || receipt.operation_roster_sha256 != manifest.operation_roster_sha256
        || receipt.policy_artifact_sha256 != manifest.policy_artifact_sha256
        || receipt.memory_system_sha256 != manifest.memory_system_sha256
        || receipt.evaluator_sha256 != manifest.evaluator_sha256
        || receipt.query_roster_sha256 != manifest.query_roster_sha256
        || receipt.control_manifest_sha256 != manifest.control_manifest_sha256
    {
        return Err(MemoryOperationError::ReceiptContextMismatch);
    }
    if receipt.case_id != case.case_id
        || receipt.task_id != case.task_id
        || receipt.episode_id != case.episode_id
        || receipt.independence_unit_id != case.independence_unit_id
        || receipt.sequence != case.sequence
        || receipt.operation != case.operation
        || receipt.observed_query_sha256.as_deref()
            .is_some_and(|query| query != case.query_sha256)
    {
        return Err(MemoryOperationError::ReceiptIdentityMismatch);
    }
    Ok(())
}

fn derive_outcome(
    case: &ExpectedMemoryOperation,
    receipt: &MemoryOperationReceipt,
) -> MemoryOperationOutcome {
    if !receipt.valid_run
        || receipt.observed_source_event_sha256.is_none()
        || receipt.observed_effect_sha256.is_none()
        || receipt.memory_snapshot_before_sha256.is_none()
        || receipt.memory_snapshot_after_sha256.is_none()
        || receipt.evaluator_receipt_sha256.is_none()
        || receipt.observed_query_sha256.is_none()
    {
        return MemoryOperationOutcome::Unresolved;
    }

    let expected_effect = case.expected_effect_sha256.as_deref();
    let observed_effect = receipt.observed_effect_sha256.as_deref();
    let source_matches = receipt.observed_source_event_sha256.as_deref()
        == Some(case.source_event_sha256.as_str());

    let query_matches = receipt.observed_query_sha256.as_deref()
        == Some(case.query_sha256.as_str());
    if source_matches && query_matches && observed_effect == expected_effect {
        MemoryOperationOutcome::Correct
    } else {
        MemoryOperationOutcome::Incorrect
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: &str) -> String {
        value.repeat(64)
    }

    fn case(
        case_id: &str,
        sequence: u64,
        operation: MemoryOperationKind,
        applicability: MemoryOperationApplicability,
        expected_effect: Option<String>,
    ) -> ExpectedMemoryOperation {
        ExpectedMemoryOperation {
            case_id: case_id.to_owned(),
            task_id: "WA-01".to_owned(),
            episode_id: "episode-001".to_owned(),
            independence_unit_id: "scene-seed-1050".to_owned(),
            sequence,
            operation,
            applicability,
            source_event_sha256: digest("1"),
            query_sha256: digest("a"),
            expected_effect_sha256: expected_effect,
        }
    }

    fn manifest(cases: Vec<ExpectedMemoryOperation>) -> MemoryOperationManifest {
        MemoryOperationManifest {
            protocol_sha256: digest("2"),
            operation_roster_sha256: digest("3"),
            policy_artifact_sha256: digest("4"),
            memory_system_sha256: digest("5"),
            evaluator_sha256: digest("6"),
            query_roster_sha256: digest("7"),
            control_manifest_sha256: digest("8"),
            expected_case_count: cases.len(),
            expected_task_ids: {
                let tasks = cases.iter().map(|case| case.task_id.clone()).collect::<BTreeSet<_>>();
                tasks.into_iter().collect()
            },
            cases,
        }
    }

    fn receipt(case: &ExpectedMemoryOperation) -> MemoryOperationReceipt {
        MemoryOperationReceipt {
            case_id: case.case_id.clone(),
            task_id: case.task_id.clone(),
            episode_id: case.episode_id.clone(),
            independence_unit_id: case.independence_unit_id.clone(),
            sequence: case.sequence,
            operation: case.operation,
            protocol_sha256: digest("2"),
            operation_roster_sha256: digest("3"),
            policy_artifact_sha256: digest("4"),
            memory_system_sha256: digest("5"),
            evaluator_sha256: digest("6"),
            query_roster_sha256: digest("7"),
            control_manifest_sha256: digest("8"),
            observed_source_event_sha256: Some(case.source_event_sha256.clone()),
            observed_query_sha256: Some(case.query_sha256.clone()),
            observed_effect_sha256: case.expected_effect_sha256.clone(),
            memory_snapshot_before_sha256: Some(digest("7")),
            memory_snapshot_after_sha256: Some(digest("8")),
            evaluator_receipt_sha256: Some(digest("9")),
            memory_bytes_before: Some(1024),
            memory_bytes_after: Some(512),
            latency_ms: Some(12),
            valid_run: true,
        }
    }

    #[test]
    fn storage_update_and_compression_are_reported_separately() {
        let cases = vec![
            case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a"))),
            case("update", 2, MemoryOperationKind::Update, MemoryOperationApplicability::Applicable, Some(digest("b"))),
            case("compress", 3, MemoryOperationKind::Compression, MemoryOperationApplicability::Applicable, Some(digest("c"))),
        ];
        let receipts = cases.iter().map(receipt).collect::<Vec<_>>();
        let audit = audit_memory_operations(&manifest(cases), &receipts).unwrap();

        assert_eq!(audit.results.len(), 3);
        assert_eq!(audit.summaries.len(), 3);
        assert_eq!(audit.task_summaries.len(), 3);
        assert!(audit.summaries.iter().all(|summary| summary.applicable_task_count == 1));
        assert!(audit.summaries.iter().all(|summary| summary.task_macro_fidelity_rate == Some(1.0)));
        for summary in &audit.summaries {
            assert_eq!(summary.applicable_count, 1);
            assert_eq!(summary.correct_count, 1);
            assert_eq!(summary.fidelity_rate, Some(1.0));
            assert_eq!(summary.evidence_coverage, Some(1.0));
        }
        let compression = audit.summaries.iter()
            .find(|row| row.operation == MemoryOperationKind::Compression).unwrap();
        assert_eq!(compression.compression_ratio, Some(0.5));
        assert_eq!(compression.compression_measurement_count, 1);
    }

    #[test]
    fn update_that_retains_a_stale_effect_is_incorrect_not_unresolved() {
        let item = case("update", 1, MemoryOperationKind::Update, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.observed_effect_sha256 = Some(digest("b"));
        let audit = audit_memory_operations(&manifest(vec![item]), &[observed]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Incorrect);
        assert_eq!(audit.summaries[0].incorrect_count, 1);
        assert_eq!(audit.summaries[0].fidelity_rate, Some(0.0));
    }

    #[test]
    fn incorrect_source_lineage_is_not_credited_as_correct_memory() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.observed_source_event_sha256 = Some(digest("b"));
        let audit = audit_memory_operations(&manifest(vec![item]), &[observed]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Incorrect);
    }

    #[test]
    fn missing_receipt_remains_unresolved_in_applicable_denominator() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let audit = audit_memory_operations(&manifest(vec![item]), &[]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Unresolved);
        assert_eq!(audit.summaries[0].applicable_count, 1);
        assert_eq!(audit.summaries[0].unresolved_count, 1);
        assert_eq!(audit.summaries[0].fidelity_rate, Some(0.0));
        assert_eq!(audit.summaries[0].evidence_coverage, Some(0.0));
    }

    #[test]
    fn invalid_run_and_missing_evaluator_readback_are_unresolved() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut invalid = receipt(&item);
        invalid.valid_run = false;
        let mut no_readback = receipt(&item);
        no_readback.observed_effect_sha256 = None;
        let audit = audit_memory_operations(&manifest(vec![item.clone()]), &[invalid]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Unresolved);
        let audit = audit_memory_operations(&manifest(vec![item]), &[no_readback]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Unresolved);
    }

    #[test]
    fn not_applicable_cases_do_not_enter_operation_denominator() {
        let item = case("compress-na", 1, MemoryOperationKind::Compression, MemoryOperationApplicability::NotApplicable, None);
        let audit = audit_memory_operations(&manifest(vec![item]), &[]).unwrap();
        assert_eq!(audit.summaries[0].not_applicable_count, 1);
        assert_eq!(audit.summaries[0].applicable_count, 0);
        assert_eq!(audit.summaries[0].fidelity_rate, None);
    }

    #[test]
    fn receipt_for_not_applicable_case_is_rejected() {
        let item = case("compress-na", 1, MemoryOperationKind::Compression, MemoryOperationApplicability::NotApplicable, None);
        let mut observed = receipt(&item);
        observed.observed_effect_sha256 = None;
        assert_eq!(
            audit_memory_operations(&manifest(vec![item]), &[observed]),
            Err(MemoryOperationError::ReceiptForNotApplicableCase)
        );
    }

    #[test]
    fn duplicate_expected_case_and_duplicate_sequence_reject() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        assert_eq!(
            audit_memory_operations(&manifest(vec![item.clone(), item.clone()]), &[]),
            Err(MemoryOperationError::DuplicateCase)
        );

        let first = item.clone();
        let mut second = case("update", 2, MemoryOperationKind::Update, MemoryOperationApplicability::Applicable, Some(digest("b")));
        second.sequence = first.sequence;
        assert_eq!(
            audit_memory_operations(&manifest(vec![first, second]), &[]),
            Err(MemoryOperationError::DuplicateSequence)
        );
    }

    #[test]
    fn manifest_sequence_must_be_monotonic_within_episode() {
        let first = case("first", 2, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let second = case("second", 1, MemoryOperationKind::Update, MemoryOperationApplicability::Applicable, Some(digest("b")));
        assert_eq!(
            audit_memory_operations(&manifest(vec![first, second]), &[]),
            Err(MemoryOperationError::NonMonotonicSequence)
        );
    }

    #[test]
    fn duplicate_unknown_and_mismatched_receipts_reject() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let observed = receipt(&item);
        assert_eq!(
            audit_memory_operations(&manifest(vec![item.clone()]), &[observed.clone(), observed.clone()]),
            Err(MemoryOperationError::DuplicateReceipt)
        );

        let mut unknown = receipt(&item);
        unknown.case_id = "other".to_owned();
        assert_eq!(
            audit_memory_operations(&manifest(vec![item.clone()]), &[unknown]),
            Err(MemoryOperationError::UnknownReceipt)
        );

        let mut mismatch = receipt(&item);
        mismatch.task_id = "TM-02".to_owned();
        assert_eq!(
            audit_memory_operations(&manifest(vec![item]), &[mismatch]),
            Err(MemoryOperationError::ReceiptIdentityMismatch)
        );
    }

    #[test]
    fn missing_snapshot_or_evaluator_receipt_withholds_correctness() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut missing_before = receipt(&item);
        missing_before.memory_snapshot_before_sha256 = None;
        let audit = audit_memory_operations(&manifest(vec![item.clone()]), &[missing_before]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Unresolved);

        let mut missing_receipt = receipt(&item);
        missing_receipt.evaluator_receipt_sha256 = None;
        let audit = audit_memory_operations(&manifest(vec![item]), &[missing_receipt]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Unresolved);
    }

    #[test]
    fn malformed_manifest_and_receipt_digests_reject() {
        let mut item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        item.expected_effect_sha256 = Some("bad".to_owned());
        assert_eq!(
            audit_memory_operations(&manifest(vec![item]), &[]),
            Err(MemoryOperationError::InvalidDigest)
        );

        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.observed_effect_sha256 = Some("bad".to_owned());
        assert_eq!(
            audit_memory_operations(&manifest(vec![item]), &[observed]),
            Err(MemoryOperationError::InvalidDigest)
        );
    }

    #[test]
    fn applicable_and_not_applicable_expected_effect_contracts_are_explicit() {
        let mut no_effect = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, None);
        assert_eq!(
            audit_memory_operations(&manifest(vec![no_effect.clone()]), &[]),
            Err(MemoryOperationError::ApplicableCaseMissingExpectedEffect)
        );
        no_effect.applicability = MemoryOperationApplicability::NotApplicable;
        no_effect.expected_effect_sha256 = Some(digest("a"));
        assert_eq!(
            audit_memory_operations(&manifest(vec![no_effect]), &[]),
            Err(MemoryOperationError::NotApplicableCaseHasExpectedEffect)
        );
    }

    #[test]
    fn memory_operation_manifest_must_be_nonempty_and_count_exact() {
        let empty = manifest(Vec::new());
        assert_eq!(audit_memory_operations(&empty, &[]), Err(MemoryOperationError::EmptyManifest));

        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut wrong_count = manifest(vec![item]);
        wrong_count.expected_case_count = 2;
        assert_eq!(
            audit_memory_operations(&wrong_count, &[]),
            Err(MemoryOperationError::ExpectedCaseCountMismatch)
        );
    }

    #[test]
    fn independent_units_can_span_multiple_operation_cases_without_pseudoreplication() {
        let mut store = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut update = case("update", 2, MemoryOperationKind::Update, MemoryOperationApplicability::Applicable, Some(digest("b")));
        store.independence_unit_id = "scene-seed-1".to_owned();
        update.independence_unit_id = "scene-seed-1".to_owned();
        let receipts = [receipt(&store), receipt(&update)];
        let audit = audit_memory_operations(&manifest(vec![store, update]), &receipts).unwrap();
        assert_eq!(audit.results.len(), 2);
        assert!(audit.results.iter().all(|result| result.independence_unit_id == "scene-seed-1"));
    }

    #[test]
    fn evaluator_query_identity_is_pinned_and_wrong_query_cannot_pass() {
        let item = case("compress", 1, MemoryOperationKind::Compression, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.observed_query_sha256 = Some(digest("b"));
        assert_eq!(
            audit_memory_operations(&manifest(vec![item]), &[observed]),
            Err(MemoryOperationError::ReceiptIdentityMismatch)
        );
    }

    #[test]
    fn correlated_operation_rows_report_distinct_independence_units() {
        let mut first = case("store-1", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut second = case("store-2", 2, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("b")));
        first.independence_unit_id = "same-scene-seed".to_owned();
        second.independence_unit_id = "same-scene-seed".to_owned();
        let receipts = [receipt(&first), receipt(&second)];
        let audit = audit_memory_operations(&manifest(vec![first, second]), &receipts).unwrap();
        assert_eq!(audit.summaries[0].applicable_count, 2);
        assert_eq!(audit.summaries[0].independent_unit_count, 1);
    }

    #[test]
    fn zero_byte_compression_baseline_is_not_divided_by_zero() {
        let item = case("compress", 1, MemoryOperationKind::Compression, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.memory_bytes_before = Some(0);
        observed.memory_bytes_after = Some(0);
        let audit = audit_memory_operations(&manifest(vec![item]), &[observed]).unwrap();
        assert_eq!(audit.summaries[0].compression_ratio, None);
        assert_eq!(audit.summaries[0].compression_measurement_count, 0);
    }

    #[test]
    fn sequence_numbers_are_scoped_to_task_and_episode_identity() {
        let first = case("task-a-store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut second = case("task-b-store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("b")));
        second.task_id = "TM-02".to_owned();
        // The episode labels may coincide across tasks; each task/episode pair
        // is a distinct event stream.
        let audit = audit_memory_operations(
            &manifest(vec![first.clone(), second.clone()]),
            &[receipt(&first), receipt(&second)],
        ).unwrap();
        assert_eq!(audit.results.len(), 2);
        // Independence identity is scoped by task as well as scene/seed label.
        assert_eq!(audit.summaries[0].independent_unit_count, 2);

        assert!(audit_memory_operations(&manifest(vec![first, second]), &[]).is_ok());
    }

    fn three_unit_paired_arms() -> (MemoryOperationPairedComparison, MemoryOperationPairedComparison) {
        let mut cases = Vec::new();
        for task_id in ["WA-01", "TM-02"] {
            for index in 0..3 {
                let mut item = case(
                    &format!("{task_id}-store-{index}"),
                    1,
                    MemoryOperationKind::Storage,
                    MemoryOperationApplicability::Applicable,
                    Some(digest("a")),
                );
                item.task_id = task_id.to_owned();
                item.episode_id = format!("{task_id}-episode-{index}");
                item.independence_unit_id = format!("{task_id}-scene-seed-{index}");
                cases.push(item);
            }
        }
        let base_manifest = manifest(cases.clone());
        let mut baseline_receipts = cases.iter().map(receipt).collect::<Vec<_>>();
        for row in &mut baseline_receipts {
            if row.case_id.ends_with("-0") {
                row.observed_effect_sha256 = Some(digest("x"));
            }
        }
        let baseline = audit_memory_operations(&base_manifest, &baseline_receipts).unwrap();

        let mut candidate_manifest = base_manifest;
        candidate_manifest.memory_system_sha256 = digest("a");
        let mut candidate_receipts = cases.iter().map(receipt).collect::<Vec<_>>();
        for row in &mut candidate_receipts {
            row.memory_system_sha256 = digest("a");
        }
        let candidate = audit_memory_operations(&candidate_manifest, &candidate_receipts).unwrap();
        let paired = compare_memory_operation_arms(&baseline, &candidate).unwrap();
        (paired, baseline)
    }

    #[test]
    fn task_stratified_cluster_bootstrap_is_deterministic_and_reports_interval() {
        let (comparison, _) = three_unit_paired_arms();
        let config = ClusterBootstrapConfig {
            replicates: 2_000,
            seed: 42,
            confidence_level: 0.95,
            minimum_independent_units_per_task: 2,
        };
        let first = bootstrap_memory_operation_comparison(&comparison, config).unwrap();
        let second = bootstrap_memory_operation_comparison(&comparison, config).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.summaries.len(), 1);
        let summary = &first.summaries[0];
        assert_eq!(summary.status, ClusterBootstrapStatus::SufficientIndependentUnits);
        assert_eq!(summary.contributing_task_count, 2);
        assert_eq!(summary.independent_unit_count, 6);
        assert_eq!(summary.estimate_task_macro_mean_delta, Some(1.0 / 3.0));
        assert!(summary.lower_percentile_bound.unwrap() <= summary.estimate_task_macro_mean_delta.unwrap());
        assert!(summary.upper_percentile_bound.unwrap() >= summary.estimate_task_macro_mean_delta.unwrap());
    }

    #[test]
    fn cluster_bootstrap_with_too_few_units_is_explicitly_underpowered() {
        let (comparison, _) = three_unit_paired_arms();
        let config = ClusterBootstrapConfig {
            replicates: 2_000,
            seed: 7,
            confidence_level: 0.95,
            minimum_independent_units_per_task: 4,
        };
        let report = bootstrap_memory_operation_comparison(&comparison, config).unwrap();
        assert_eq!(report.summaries[0].status, ClusterBootstrapStatus::Underpowered);
        assert_eq!(report.summaries[0].estimate_task_macro_mean_delta, Some(1.0 / 3.0));
        assert_eq!(report.summaries[0].lower_percentile_bound, None);
        assert_eq!(report.summaries[0].upper_percentile_bound, None);
    }

    #[test]
    fn cluster_bootstrap_rejects_invalid_configuration() {
        let (comparison, _) = three_unit_paired_arms();
        let mut config = ClusterBootstrapConfig {
            replicates: 0,
            seed: 1,
            confidence_level: 0.95,
            minimum_independent_units_per_task: 2,
        };
        assert_eq!(
            bootstrap_memory_operation_comparison(&comparison, config),
            Err(MemoryOperationError::BootstrapInvalidConfig)
        );
        config.replicates = 1_000;
        config.confidence_level = 1.0;
        assert_eq!(
            bootstrap_memory_operation_comparison(&comparison, config),
            Err(MemoryOperationError::BootstrapInvalidConfig)
        );
        config.confidence_level = 0.95;
        config.minimum_independent_units_per_task = 1;
        assert_eq!(
            bootstrap_memory_operation_comparison(&comparison, config),
            Err(MemoryOperationError::BootstrapInvalidConfig)
        );
    }

    #[test]
    fn paired_memory_comparison_requires_exact_control_and_policy_identity() {
        let first = case("store-1", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut second = case("store-2", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("b")));
        second.episode_id = "episode-002".to_owned();
        second.independence_unit_id = "scene-seed-1051".to_owned();
        let base_manifest = manifest(vec![first.clone(), second.clone()]);
        let base_receipts = {
            let mut rows = vec![receipt(&first), receipt(&second)];
            rows[1].observed_effect_sha256 = Some(digest("x"));
            rows
        };
        let baseline = audit_memory_operations(&base_manifest, &base_receipts).unwrap();

        let mut candidate_manifest = base_manifest.clone();
        candidate_manifest.memory_system_sha256 = digest("a");
        let mut candidate_receipts = vec![receipt(&first), receipt(&second)];
        for row in &mut candidate_receipts {
            row.memory_system_sha256 = digest("a");
        }
        let candidate = audit_memory_operations(&candidate_manifest, &candidate_receipts).unwrap();
        let paired = compare_memory_operation_arms(&baseline, &candidate).unwrap();
        assert_eq!(paired.paired_unit_count, 2);
        assert_eq!(paired.summaries.len(), 1);
        assert_eq!(paired.summaries[0].candidate_wins, 1);
        assert_eq!(paired.summaries[0].ties, 1);
        assert_eq!(paired.summaries[0].mean_paired_fidelity_delta, Some(0.5));

        let mut mismatch = candidate.clone();
        mismatch.control_manifest_sha256 = digest("f");
        assert_eq!(
            compare_memory_operation_arms(&baseline, &mismatch),
            Err(MemoryOperationError::ComparisonControlManifestMismatch)
        );
        mismatch = candidate;
        mismatch.policy_artifact_sha256 = digest("f");
        assert_eq!(
            compare_memory_operation_arms(&baseline, &mismatch),
            Err(MemoryOperationError::ComparisonPolicyArtifactMismatch)
        );
    }

    #[test]
    fn one_episode_cannot_claim_multiple_independence_units() {
        let first = case("store-1", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut second = case("store-2", 2, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("b")));
        second.independence_unit_id = "other-scene-seed".to_owned();
        assert_eq!(
            audit_memory_operations(&manifest(vec![first, second]), &[]),
            Err(MemoryOperationError::EpisodeIndependenceUnitMismatch)
        );
    }

    #[test]
    fn paired_memory_comparison_rejects_different_task_populations() {
        let first = case("store-a", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let baseline = audit_memory_operations(
            &manifest(vec![first.clone()]),
            &[receipt(&first)],
        ).unwrap();

        let mut candidate_case = first.clone();
        candidate_case.task_id = "TM-02".to_owned();
        let candidate = audit_memory_operations(
            &manifest(vec![candidate_case.clone()]),
            &[receipt(&candidate_case)],
        ).unwrap();

        assert_eq!(
            compare_memory_operation_arms(&baseline, &candidate),
            Err(MemoryOperationError::ComparisonTaskRosterMismatch)
        );
    }

    #[test]
    fn paired_memory_comparison_rejects_different_independent_unit_rosters() {
        let first = case("store-1", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut second = case("store-2", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("b")));
        second.episode_id = "episode-002".to_owned();
        second.independence_unit_id = "scene-seed-1051".to_owned();
        let baseline_manifest = manifest(vec![first.clone(), second.clone()]);
        let baseline = audit_memory_operations(
            &baseline_manifest,
            &[receipt(&first), receipt(&second)],
        ).unwrap();

        let mut changed = second.clone();
        changed.independence_unit_id = "different-scene-seed".to_owned();
        let candidate_manifest = manifest(vec![first.clone(), changed.clone()]);
        let candidate = audit_memory_operations(
            &candidate_manifest,
            &[receipt(&first), receipt(&changed)],
        ).unwrap();
        assert_eq!(
            compare_memory_operation_arms(&baseline, &candidate),
            Err(MemoryOperationError::ComparisonUnitRosterMismatch)
        );
    }

    #[test]
    fn per_independence_unit_rows_keep_correlated_checkpoints_together() {
        let mut first = case("store-1", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut second = case("store-2", 2, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("b")));
        first.independence_unit_id = "shared-scene-seed".to_owned();
        second.independence_unit_id = "shared-scene-seed".to_owned();
        let mut second_receipt = receipt(&second);
        second_receipt.observed_effect_sha256 = Some(digest("x"));
        let audit = audit_memory_operations(
            &manifest(vec![first.clone(), second.clone()]),
            &[receipt(&first), second_receipt],
        ).unwrap();

        assert_eq!(audit.unit_summaries.len(), 1);
        assert_eq!(audit.unit_summaries[0].independence_unit_id, "shared-scene-seed");
        assert_eq!(audit.unit_summaries[0].applicable_count, 2);
        assert_eq!(audit.unit_summaries[0].correct_count, 1);
        assert_eq!(audit.unit_summaries[0].incorrect_count, 1);
        assert_eq!(audit.unit_summaries[0].fidelity_rate, Some(0.5));
    }

    #[test]
    fn missing_evaluator_query_readback_remains_unresolved() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.observed_query_sha256 = None;
        let audit = audit_memory_operations(&manifest(vec![item]), &[observed]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Unresolved);
    }

    #[test]
    fn latency_and_compression_measurements_have_explicit_denominators() {
        let item = case("compress", 1, MemoryOperationKind::Compression, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.latency_ms = None;
        observed.memory_bytes_after = None;
        let audit = audit_memory_operations(&manifest(vec![item]), &[observed]).unwrap();
        assert_eq!(audit.summaries[0].latency_measurement_count, 0);
        assert_eq!(audit.summaries[0].compression_measurement_count, 0);
        assert_eq!(audit.summaries[0].compression_ratio, None);
    }

    #[test]
    fn task_stratification_prevents_easy_task_from_hiding_memory_failures() {
        let easy = case("store-easy", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut hard = case("store-hard", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("b")));
        hard.task_id = "TM-02".to_owned();
        let mut hard_receipt = receipt(&hard);
        hard_receipt.observed_effect_sha256 = Some(digest("x"));
        let audit = audit_memory_operations(
            &manifest(vec![easy.clone(), hard.clone()]),
            &[receipt(&easy), hard_receipt],
        ).unwrap();

        assert_eq!(audit.summaries[0].fidelity_rate, Some(0.5));
        assert_eq!(audit.summaries[0].task_macro_fidelity_rate, Some(0.5));
        assert_eq!(audit.task_summaries.len(), 2);
        assert_eq!(audit.task_summaries[0].fidelity_rate, Some(1.0));
        assert_eq!(audit.task_summaries[1].fidelity_rate, Some(0.0));
    }

    #[test]
    fn manifest_rejects_missing_duplicate_and_unexpected_task_roster_entries() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));

        let mut missing = manifest(vec![item.clone()]);
        missing.expected_task_ids.clear();
        assert_eq!(
            audit_memory_operations(&missing, &[]),
            Err(MemoryOperationError::EmptyTaskRoster)
        );

        let mut duplicate = manifest(vec![item.clone()]);
        duplicate.expected_task_ids.push("WA-01".to_owned());
        assert_eq!(
            audit_memory_operations(&duplicate, &[]),
            Err(MemoryOperationError::DuplicateTask)
        );

        let mut unexpected = manifest(vec![item.clone()]);
        unexpected.expected_task_ids = vec!["TM-02".to_owned()];
        assert_eq!(
            audit_memory_operations(&unexpected, &[]),
            Err(MemoryOperationError::UnexpectedTask)
        );

        let mut omitted_task = manifest(vec![item]);
        omitted_task.expected_task_ids.push("TM-02".to_owned());
        omitted_task.expected_case_count = 2;
        assert_eq!(
            audit_memory_operations(&omitted_task, &[]),
            Err(MemoryOperationError::TaskSetMismatch)
        );
    }

    #[test]
    fn receipt_context_must_match_protocol_policy_memory_and_evaluator() {
        let item = case("store", 1, MemoryOperationKind::Storage, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let mut observed = receipt(&item);
        observed.memory_system_sha256 = digest("z");
        assert_eq!(
            audit_memory_operations(&manifest(vec![item]), &[observed]),
            Err(MemoryOperationError::ReceiptContextMismatch)
        );
    }

    #[test]
    fn semantically_faithful_compression_does_not_require_identical_memory_bytes() {
        let item = case("compress", 1, MemoryOperationKind::Compression, MemoryOperationApplicability::Applicable, Some(digest("required-facts")));
        let mut observed = receipt(&item);
        observed.observed_effect_sha256 = Some(digest("required-facts"));
        observed.memory_snapshot_before_sha256 = Some(digest("uncompressed-layout"));
        observed.memory_snapshot_after_sha256 = Some(digest("compressed-layout"));
        observed.memory_bytes_before = Some(4096);
        observed.memory_bytes_after = Some(1024);

        let audit = audit_memory_operations(&manifest(vec![item]), &[observed]).unwrap();
        assert_eq!(audit.results[0].outcome, MemoryOperationOutcome::Correct);
        assert_eq!(audit.summaries[0].compression_ratio, Some(0.25));
    }

    #[test]
    fn task_and_memory_operation_identity_are_preserved_in_audit() {
        let item = case("update", 1, MemoryOperationKind::Update, MemoryOperationApplicability::Applicable, Some(digest("a")));
        let receipt = receipt(&item);
        let audit = audit_memory_operations(&manifest(vec![item]), &[receipt]).unwrap();
        assert_eq!(audit.results[0].task_id, "WA-01");
        assert_eq!(audit.results[0].operation, MemoryOperationKind::Update);
        assert_eq!(audit.results[0].independence_unit_id, "scene-seed-1050");
    }
}
