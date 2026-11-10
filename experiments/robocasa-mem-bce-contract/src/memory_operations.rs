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
    ComparisonTaskRosterMismatch,
    ComparisonApplicabilityMismatch,
    ComparisonUnitRosterMismatch,
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
    independent_units: BTreeSet<String>,
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
        self.independent_units.insert(result.independence_unit_id.clone());

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
        assert_eq!(audit.summaries[0].independent_unit_count, 1);

        assert!(audit_memory_operations(&manifest(vec![first, second]), &[]).is_ok());
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
