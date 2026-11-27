//! Fail-closed qualification semantics for Offline Cold-Start and Trust Closure.
//!
//! This crate scores observations produced by an independent test harness. It
//! does not verify signatures, hashes, sandboxing, package completeness, or
//! reader independence itself. Those measurements must be supplied by
//! independently controlled tools and retained in the evidence receipt.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Pass,
    Fail,
    Incomplete,
    NotEvaluated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustAnchorSource {
    OutOfBand,
    ArchiveOnly,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationDisposition {
    Pass,
    Fail,
    Incomplete,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialReceipt {
    pub scenario_id: String,
    pub bitstream_integrity: EvidenceStatus,
    pub authenticity: EvidenceStatus,
    pub interpretability: EvidenceStatus,
    pub operational_closure: EvidenceStatus,
    pub expected_digest_source: TrustAnchorSource,
    pub signature_trust_root_source: TrustAnchorSource,
    /// Stable independent implementation IDs, not display names.
    pub reader_ids: Vec<String>,
    /// Produced by an independent dependency/common-cause assessment.
    pub reader_independence: EvidenceStatus,
    /// Exact byte and semantic output comparison between reader paths.
    pub reader_agreement: EvidenceStatus,
    pub preservation_objectives_required: usize,
    pub preservation_objectives_passed: usize,
    pub undeclared_runtime_dependencies: usize,
    pub attempted_network_calls: usize,
    pub sandbox_violations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptContractError {
    EmptyScenarioId,
    EmptyReaderId,
    DuplicateReaderId,
    ObjectivePassCountExceedsRequired,
}

fn observed_statuses(receipt: &TrialReceipt) -> [EvidenceStatus; 6] {
    [
        receipt.bitstream_integrity,
        receipt.authenticity,
        receipt.interpretability,
        receipt.operational_closure,
        receipt.reader_independence,
        receipt.reader_agreement,
    ]
}

/// Score a harness-produced receipt. Candidate declarations are not evidence:
/// statuses, dependency counts, network traces and sandbox events must come
/// from the external qualification harness.
pub fn qualify(
    receipt: &TrialReceipt,
) -> Result<QualificationDisposition, ReceiptContractError> {
    if receipt.scenario_id.trim().is_empty() {
        return Err(ReceiptContractError::EmptyScenarioId);
    }

    for (index, reader_id) in receipt.reader_ids.iter().enumerate() {
        if reader_id.trim().is_empty() {
            return Err(ReceiptContractError::EmptyReaderId);
        }
        if receipt.reader_ids[..index].iter().any(|seen| seen == reader_id) {
            return Err(ReceiptContractError::DuplicateReaderId);
        }
    }

    if receipt.preservation_objectives_passed > receipt.preservation_objectives_required {
        return Err(ReceiptContractError::ObjectivePassCountExceedsRequired);
    }

    let statuses = observed_statuses(receipt);
    if statuses.iter().all(|status| *status == EvidenceStatus::NotEvaluated) {
        return Ok(QualificationDisposition::NotEvaluated);
    }

    // Hard failures cannot be averaged away by successes in another dimension.
    if statuses.iter().any(|status| *status == EvidenceStatus::Fail)
        || receipt.undeclared_runtime_dependencies > 0
        || receipt.attempted_network_calls > 0
        || receipt.sandbox_violations > 0
    {
        return Ok(QualificationDisposition::Fail);
    }

    // A candidate cannot use a digest or key carried only by the candidate
    // archive as an independent integrity/authenticity trust anchor.
    if receipt.bitstream_integrity == EvidenceStatus::Pass
        && receipt.expected_digest_source != TrustAnchorSource::OutOfBand
    {
        return Ok(QualificationDisposition::Fail);
    }
    if receipt.authenticity == EvidenceStatus::Pass
        && receipt.signature_trust_root_source != TrustAnchorSource::OutOfBand
    {
        return Ok(QualificationDisposition::Fail);
    }

    // An asserted interpretability pass must cover every declared objective.
    if receipt.interpretability == EvidenceStatus::Pass
        && (receipt.preservation_objectives_required == 0
            || receipt.preservation_objectives_passed
                != receipt.preservation_objectives_required)
    {
        return Ok(QualificationDisposition::Fail);
    }

    // A full preservation claim requires two separately identified readers
    // and an explicit independent common-cause/dependency assessment.
    if receipt.reader_ids.len() < 2 || receipt.preservation_objectives_required == 0 {
        return Ok(QualificationDisposition::Incomplete);
    }

    if statuses.iter().any(|status| {
        *status == EvidenceStatus::Incomplete || *status == EvidenceStatus::NotEvaluated
    }) {
        return Ok(QualificationDisposition::Incomplete);
    }

    if statuses.iter().all(|status| *status == EvidenceStatus::Pass)
        && receipt.expected_digest_source == TrustAnchorSource::OutOfBand
        && receipt.signature_trust_root_source == TrustAnchorSource::OutOfBand
        && receipt.preservation_objectives_passed
            == receipt.preservation_objectives_required
        && receipt.undeclared_runtime_dependencies == 0
        && receipt.attempted_network_calls == 0
        && receipt.sandbox_violations == 0
    {
        return Ok(QualificationDisposition::Pass);
    }

    Ok(QualificationDisposition::Incomplete)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_receipt() -> TrialReceipt {
        TrialReceipt {
            scenario_id: "COLD-01".into(),
            bitstream_integrity: EvidenceStatus::Pass,
            authenticity: EvidenceStatus::Pass,
            interpretability: EvidenceStatus::Pass,
            operational_closure: EvidenceStatus::Pass,
            expected_digest_source: TrustAnchorSource::OutOfBand,
            signature_trust_root_source: TrustAnchorSource::OutOfBand,
            reader_ids: vec!["reader-rust-independent-a".into(), "reader-c-independent-b".into()],
            reader_independence: EvidenceStatus::Pass,
            reader_agreement: EvidenceStatus::Pass,
            preservation_objectives_required: 2,
            preservation_objectives_passed: 2,
            undeclared_runtime_dependencies: 0,
            attempted_network_calls: 0,
            sandbox_violations: 0,
        }
    }

    #[test]
    fn fully_qualified_offline_trial_passes() {
        assert_eq!(qualify(&passing_receipt()), Ok(QualificationDisposition::Pass));
    }

    #[test]
    fn missing_or_archive_only_trust_root_is_incomplete() {
        for source in [TrustAnchorSource::Missing, TrustAnchorSource::ArchiveOnly, TrustAnchorSource::Unknown] {
            let mut receipt = passing_receipt();
            receipt.signature_trust_root_source = source;
            receipt.authenticity = EvidenceStatus::Incomplete;
            assert_eq!(qualify(&receipt), Ok(QualificationDisposition::Incomplete));
        }
    }

    #[test]
    fn archive_only_digest_cannot_claim_independent_integrity() {
        let mut receipt = passing_receipt();
        receipt.expected_digest_source = TrustAnchorSource::ArchiveOnly;
        assert_eq!(qualify(&receipt), Ok(QualificationDisposition::Fail));
    }

    #[test]
    fn hidden_dependencies_network_and_sandbox_violations_are_hard_failures() {
        let mut dependency = passing_receipt();
        dependency.undeclared_runtime_dependencies = 1;
        assert_eq!(qualify(&dependency), Ok(QualificationDisposition::Fail));

        let mut network = passing_receipt();
        network.attempted_network_calls = 1;
        assert_eq!(qualify(&network), Ok(QualificationDisposition::Fail));

        let mut sandbox = passing_receipt();
        sandbox.sandbox_violations = 1;
        assert_eq!(qualify(&sandbox), Ok(QualificationDisposition::Fail));
    }

    #[test]
    fn reader_divergence_and_known_semantic_failure_are_hard_failures() {
        let mut readers = passing_receipt();
        readers.reader_agreement = EvidenceStatus::Fail;
        assert_eq!(qualify(&readers), Ok(QualificationDisposition::Fail));

        let mut semantics = passing_receipt();
        semantics.interpretability = EvidenceStatus::Fail;
        semantics.preservation_objectives_passed = 1;
        assert_eq!(qualify(&semantics), Ok(QualificationDisposition::Fail));
    }

    #[test]
    fn one_reader_or_unassessed_independence_is_incomplete() {
        let mut one_reader = passing_receipt();
        one_reader.reader_ids.pop();
        assert_eq!(qualify(&one_reader), Ok(QualificationDisposition::Incomplete));

        let mut shared_stack = passing_receipt();
        shared_stack.reader_independence = EvidenceStatus::Incomplete;
        assert_eq!(qualify(&shared_stack), Ok(QualificationDisposition::Incomplete));
    }

    #[test]
    fn missing_representation_information_is_incomplete() {
        let mut receipt = passing_receipt();
        receipt.interpretability = EvidenceStatus::Incomplete;
        receipt.preservation_objectives_passed = 1;
        assert_eq!(qualify(&receipt), Ok(QualificationDisposition::Incomplete));
    }

    #[test]
    fn zero_preservation_objectives_never_proves_understandability() {
        let mut receipt = passing_receipt();
        receipt.preservation_objectives_required = 0;
        receipt.preservation_objectives_passed = 0;
        receipt.interpretability = EvidenceStatus::Incomplete;
        assert_eq!(qualify(&receipt), Ok(QualificationDisposition::Incomplete));
    }

    #[test]
    fn all_not_evaluated_is_not_evaluated() {
        let mut receipt = passing_receipt();
        receipt.bitstream_integrity = EvidenceStatus::NotEvaluated;
        receipt.authenticity = EvidenceStatus::NotEvaluated;
        receipt.interpretability = EvidenceStatus::NotEvaluated;
        receipt.operational_closure = EvidenceStatus::NotEvaluated;
        receipt.reader_independence = EvidenceStatus::NotEvaluated;
        receipt.reader_agreement = EvidenceStatus::NotEvaluated;
        assert_eq!(qualify(&receipt), Ok(QualificationDisposition::NotEvaluated));
    }

    #[test]
    fn duplicate_reader_ids_and_impossible_counts_are_rejected() {
        let mut duplicate = passing_receipt();
        duplicate.reader_ids[1] = duplicate.reader_ids[0].clone();
        assert_eq!(qualify(&duplicate), Err(ReceiptContractError::DuplicateReaderId));

        let mut impossible = passing_receipt();
        impossible.preservation_objectives_passed = 3;
        assert_eq!(
            qualify(&impossible),
            Err(ReceiptContractError::ObjectivePassCountExceedsRequired)
        );
    }

    #[test]
    fn asserted_pass_with_an_untrusted_signature_root_fails() {
        let mut receipt = passing_receipt();
        receipt.signature_trust_root_source = TrustAnchorSource::ArchiveOnly;
        assert_eq!(qualify(&receipt), Ok(QualificationDisposition::Fail));
    }
}
