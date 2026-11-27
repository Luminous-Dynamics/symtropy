//! Deterministic, fail-closed scoring semantics for end-to-end preservation service trials.
//!
//! The scorer distinguishes bytes read/reconstructed/delivered from bytes
//! independently accepted. It scores receipts supplied by an independent
//! harness; it does not itself read media, verify cryptographic digests,
//! authenticate provenance, meter energy, or establish physical reader quality.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Pass,
    Fail,
    Incomplete,
    NotEvaluated,
    NotRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    RequestAdmission,
    OperatorApproval,
    MediaMount,
    ReaderBootstrap,
    QueueAndStaging,
    KeyAndTrustBootstrap,
    ReaderCalibration,
    PhysicalRead,
    RetryAndEcc,
    DecodeAndReconstruct,
    PackagingAndDecrypt,
    FixityAndManifest,
    NetworkTransfer,
    DestinationVerify,
    MigrationWrite,
    MigrationReadback,
    ProvenanceAcceptance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBoundary {
    WholeService,
    StorageSubsystem,
    ReaderAndCompute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceDisposition {
    Pass,
    Fail,
    Incomplete,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadContract {
    pub workload_id: String,
    pub requested_bytes: u64,
    pub requested_objects: u64,
    pub deadline_ms: u64,
    pub deadline_required: bool,
    pub manifest_required: bool,
    pub provenance_required: bool,
    pub migration_required: bool,
    pub energy_metering_required: bool,
    /// Frozen energy boundary; None is valid only when energy is not required.
    pub required_energy_boundary: Option<EnergyBoundary>,
    pub independent_reader_paths_required: bool,
    /// Frozen by the benchmark protocol, not supplied by the candidate.
    pub required_stages: Vec<Stage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageObservation {
    pub stage: Stage,
    pub elapsed_ms: u64,
    pub evidence_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceReceipt {
    pub workload_id: String,
    pub elapsed_ms: u64,
    pub time_to_first_accepted_byte_ms: Option<u64>,
    pub reconstructed_bytes: u64,
    pub delivered_bytes: u64,
    /// Bytes accepted by the independent scorer, not the candidate.
    pub independently_accepted_bytes: u64,
    pub independently_accepted_objects: u64,
    pub missing_objects: u64,
    pub extra_objects: u64,
    pub duplicate_objects: u64,
    pub unrecoverable_objects: u64,
    pub false_acceptances: u64,
    pub expected_digest_status: EvidenceStatus,
    pub manifest_status: EvidenceStatus,
    pub provenance_status: EvidenceStatus,
    /// For migration trials, source and destination payload digests are
    /// independently checked against scorer-held expectations.
    pub source_digest_status: EvidenceStatus,
    pub destination_digest_status: EvidenceStatus,
    pub migration_lineage_status: EvidenceStatus,
    pub reader_path_ids: Vec<String>,
    pub reader_independence_status: EvidenceStatus,
    pub reader_agreement_status: EvidenceStatus,
    pub energy_meter_status: EvidenceStatus,
    pub energy_millijoules: Option<u128>,
    pub energy_meter_id: Option<String>,
    pub energy_calibration_ref: Option<String>,
    /// Digest of the raw meter samples/trace retained by the independent scorer.
    pub energy_evidence_sha256: Option<String>,
    pub energy_boundary: Option<EnergyBoundary>,
    pub energy_sample_interval_ms: Option<u64>,
    pub candidate_claimed_full_success: bool,
    pub stage_observations: Vec<StageObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceScore {
    pub disposition: ServiceDisposition,
    pub accepted_bytes_per_second: u128,
    pub deadline_met: Option<bool>,
    pub missing_required_stages: Vec<Stage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    EmptyWorkloadId,
    WorkloadIdMismatch,
    ZeroRequestedBytes,
    ZeroRequestedObjects,
    ZeroElapsedTime,
    MissingRequiredDeadline,
    NoRequiredStages,
    MissingMigrationStage,
    MigrationRequiresProvenance,
    RequiredStageDuplicated,
    ObservedStageDuplicated,
    ObjectAccountingOverflow,
    ObjectAccountingMismatch,
    AcceptedBytesExceedDelivered,
    DeliveredBytesExceedReconstructed,
    AcceptedBytesExceedRequested,
    AcceptedObjectsExceedRequested,
    TimeToFirstByteAfterCompletion,
    StageDurationExceedsElapsed,
    EmptyReaderPathId,
    DuplicateReaderPathId,
    EnergyValueWithoutMeterPass,
    MeterPassWithoutEnergyValue,
    MissingEnergyBoundaryContract,
    MissingEnergyMeterMetadata,
    InvalidEnergyEvidenceDigest,
    EnergyMeterBoundaryMismatch,
    ZeroEnergySampleInterval,
}

// Shape validation only: the harness must separately hash and authenticate the raw trace.
// The SHA-shaped value alone never certifies meter calibration or measurement integrity.
fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn contains_duplicate_stages(stages: &[Stage]) -> bool {
    stages
        .iter()
        .enumerate()
        .any(|(index, stage)| stages[..index].contains(stage))
}

fn rate(bytes: u64, elapsed_ms: u64) -> u128 {
    if elapsed_ms == 0 {
        0
    } else {
        u128::from(bytes) * 1000 / u128::from(elapsed_ms)
    }
}

/// Score a single independently observed service receipt.
///
/// Hard integrity/provenance failures, false acceptance, extra/duplicate
/// objects, missed required deadlines and accounting violations cannot be
/// averaged away. Missing evidence and incomplete restoration remain
/// incomplete; they never become a pass.
pub fn score(
    contract: &WorkloadContract,
    receipt: &ServiceReceipt,
) -> Result<ServiceScore, ContractError> {
    if contract.workload_id.trim().is_empty() {
        return Err(ContractError::EmptyWorkloadId);
    }
    if receipt.workload_id != contract.workload_id {
        return Err(ContractError::WorkloadIdMismatch);
    }
    if contract.requested_bytes == 0 {
        return Err(ContractError::ZeroRequestedBytes);
    }
    if contract.requested_objects == 0 {
        return Err(ContractError::ZeroRequestedObjects);
    }
    if receipt.elapsed_ms == 0 {
        return Err(ContractError::ZeroElapsedTime);
    }
    if contract.deadline_required && contract.deadline_ms == 0 {
        return Err(ContractError::MissingRequiredDeadline);
    }
    if contract.required_stages.is_empty() {
        return Err(ContractError::NoRequiredStages);
    }
    if contract.energy_metering_required && contract.required_energy_boundary.is_none() {
        return Err(ContractError::MissingEnergyBoundaryContract);
    }
    for (index, reader_id) in receipt.reader_path_ids.iter().enumerate() {
        if reader_id.trim().is_empty() {
            return Err(ContractError::EmptyReaderPathId);
        }
        if receipt.reader_path_ids[..index]
            .iter()
            .any(|seen| seen == reader_id)
        {
            return Err(ContractError::DuplicateReaderPathId);
        }
    }
    if contains_duplicate_stages(&contract.required_stages) {
        return Err(ContractError::RequiredStageDuplicated);
    }
    if contract.migration_required && !contract.provenance_required {
        return Err(ContractError::MigrationRequiresProvenance);
    }
    if contract.migration_required
        && [
            Stage::PhysicalRead,
            Stage::FixityAndManifest,
            Stage::MigrationWrite,
            Stage::MigrationReadback,
            Stage::DestinationVerify,
            Stage::ProvenanceAcceptance,
        ]
        .iter()
        .any(|required| !contract.required_stages.contains(required))
    {
        return Err(ContractError::MissingMigrationStage);
    }

    let observed_stages: Vec<Stage> = receipt
        .stage_observations
        .iter()
        .map(|entry| entry.stage)
        .collect();
    if contains_duplicate_stages(&observed_stages) {
        return Err(ContractError::ObservedStageDuplicated);
    }

    let object_total = receipt
        .independently_accepted_objects
        .checked_add(receipt.missing_objects)
        .ok_or(ContractError::ObjectAccountingOverflow)?;
    if object_total != contract.requested_objects {
        return Err(ContractError::ObjectAccountingMismatch);
    }
    if receipt.independently_accepted_bytes > receipt.delivered_bytes {
        return Err(ContractError::AcceptedBytesExceedDelivered);
    }
    if receipt.delivered_bytes > receipt.reconstructed_bytes {
        return Err(ContractError::DeliveredBytesExceedReconstructed);
    }
    if receipt.independently_accepted_bytes > contract.requested_bytes {
        return Err(ContractError::AcceptedBytesExceedRequested);
    }
    if receipt.independently_accepted_objects > contract.requested_objects {
        return Err(ContractError::AcceptedObjectsExceedRequested);
    }
    if receipt
        .time_to_first_accepted_byte_ms
        .is_some_and(|time| time > receipt.elapsed_ms)
    {
        return Err(ContractError::TimeToFirstByteAfterCompletion);
    }
    if receipt
        .stage_observations
        .iter()
        .any(|stage| stage.elapsed_ms > receipt.elapsed_ms)
    {
        return Err(ContractError::StageDurationExceedsElapsed);
    }
    if receipt.energy_millijoules.is_some() && receipt.energy_meter_status != EvidenceStatus::Pass {
        return Err(ContractError::EnergyValueWithoutMeterPass);
    }
    if receipt.energy_meter_status == EvidenceStatus::Pass && receipt.energy_millijoules.is_none() {
        return Err(ContractError::MeterPassWithoutEnergyValue);
    }
    if receipt.energy_meter_status == EvidenceStatus::Pass {
        if receipt.energy_meter_id.as_deref().is_none_or(str::is_empty)
            || receipt
                .energy_calibration_ref
                .as_deref()
                .is_none_or(str::is_empty)
            || receipt.energy_evidence_sha256.is_none()
            || receipt.energy_boundary.is_none()
            || receipt.energy_sample_interval_ms.is_none()
        {
            return Err(ContractError::MissingEnergyMeterMetadata);
        }
        if !is_sha256_hex(receipt.energy_evidence_sha256.as_deref().unwrap()) {
            return Err(ContractError::InvalidEnergyEvidenceDigest);
        }
        if receipt.energy_boundary != contract.required_energy_boundary {
            return Err(ContractError::EnergyMeterBoundaryMismatch);
        }
        if receipt.energy_sample_interval_ms == Some(0) {
            return Err(ContractError::ZeroEnergySampleInterval);
        }
    }

    let missing_required_stages: Vec<Stage> = contract
        .required_stages
        .iter()
        .copied()
        .filter(|required| {
            !receipt
                .stage_observations
                .iter()
                .any(|observed| observed.stage == *required && observed.evidence_present)
        })
        .collect();

    let deadline_met = if contract.deadline_required {
        Some(receipt.elapsed_ms <= contract.deadline_ms)
    } else {
        None
    };

    let accepted_bytes_per_second = rate(receipt.independently_accepted_bytes, receipt.elapsed_ms);

    let hard_evidence_failure = receipt.expected_digest_status == EvidenceStatus::Fail
        || (contract.manifest_required && receipt.manifest_status == EvidenceStatus::Fail)
        || (contract.provenance_required && receipt.provenance_status == EvidenceStatus::Fail)
        || (contract.migration_required
            && (receipt.source_digest_status == EvidenceStatus::Fail
                || receipt.destination_digest_status == EvidenceStatus::Fail
                || receipt.migration_lineage_status == EvidenceStatus::Fail))
        || (contract.independent_reader_paths_required
            && (receipt.reader_independence_status == EvidenceStatus::Fail
                || receipt.reader_agreement_status == EvidenceStatus::Fail))
        || (contract.energy_metering_required
            && receipt.energy_meter_status == EvidenceStatus::Fail);

    let hard_service_failure = receipt.false_acceptances > 0
        || receipt.extra_objects > 0
        || receipt.duplicate_objects > 0
        || deadline_met == Some(false);

    if hard_evidence_failure || hard_service_failure {
        return Ok(ServiceScore {
            disposition: ServiceDisposition::Fail,
            accepted_bytes_per_second,
            deadline_met,
            missing_required_stages,
        });
    }

    let all_not_evaluated = receipt.expected_digest_status == EvidenceStatus::NotEvaluated
        && (!contract.manifest_required || receipt.manifest_status == EvidenceStatus::NotEvaluated)
        && (!contract.provenance_required
            || receipt.provenance_status == EvidenceStatus::NotEvaluated)
        && (!contract.migration_required
            || (receipt.source_digest_status == EvidenceStatus::NotEvaluated
                && receipt.destination_digest_status == EvidenceStatus::NotEvaluated
                && receipt.migration_lineage_status == EvidenceStatus::NotEvaluated))
        && (!contract.independent_reader_paths_required
            || (receipt.reader_independence_status == EvidenceStatus::NotEvaluated
                && receipt.reader_agreement_status == EvidenceStatus::NotEvaluated))
        && (!contract.energy_metering_required
            || receipt.energy_meter_status == EvidenceStatus::NotEvaluated)
        && receipt.stage_observations.is_empty()
        && receipt.independently_accepted_bytes == 0
        && receipt.independently_accepted_objects == 0;
    if all_not_evaluated {
        return Ok(ServiceScore {
            disposition: ServiceDisposition::NotEvaluated,
            accepted_bytes_per_second,
            deadline_met,
            missing_required_stages,
        });
    }

    if receipt.candidate_claimed_full_success
        && (receipt.independently_accepted_bytes != contract.requested_bytes
            || receipt.independently_accepted_objects != contract.requested_objects
            || receipt.unrecoverable_objects > 0
            || (receipt.independently_accepted_bytes > 0
                && receipt.time_to_first_accepted_byte_ms.is_none())
            || receipt.expected_digest_status != EvidenceStatus::Pass
            || (contract.manifest_required && receipt.manifest_status != EvidenceStatus::Pass)
            || (contract.provenance_required && receipt.provenance_status != EvidenceStatus::Pass)
            || (contract.migration_required
                && (receipt.source_digest_status != EvidenceStatus::Pass
                    || receipt.destination_digest_status != EvidenceStatus::Pass
                    || receipt.migration_lineage_status != EvidenceStatus::Pass))
            || (contract.independent_reader_paths_required
                && (receipt.reader_path_ids.len() < 2
                    || receipt.reader_independence_status != EvidenceStatus::Pass
                    || receipt.reader_agreement_status != EvidenceStatus::Pass))
            || (contract.energy_metering_required
                && receipt.energy_meter_status != EvidenceStatus::Pass)
            || !missing_required_stages.is_empty())
    {
        return Ok(ServiceScore {
            disposition: ServiceDisposition::Fail,
            accepted_bytes_per_second,
            deadline_met,
            missing_required_stages,
        });
    }

    let required_evidence_incomplete = receipt.expected_digest_status != EvidenceStatus::Pass
        || (contract.manifest_required && receipt.manifest_status != EvidenceStatus::Pass)
        || (contract.provenance_required && receipt.provenance_status != EvidenceStatus::Pass)
        || (contract.migration_required
            && (receipt.source_digest_status != EvidenceStatus::Pass
                || receipt.destination_digest_status != EvidenceStatus::Pass
                || receipt.migration_lineage_status != EvidenceStatus::Pass))
        || (contract.independent_reader_paths_required
            && (receipt.reader_path_ids.len() < 2
                || receipt.reader_independence_status != EvidenceStatus::Pass
                || receipt.reader_agreement_status != EvidenceStatus::Pass))
        || (contract.energy_metering_required
            && receipt.energy_meter_status != EvidenceStatus::Pass)
        || !missing_required_stages.is_empty();

    let service_incomplete = receipt.independently_accepted_bytes != contract.requested_bytes
        || receipt.independently_accepted_objects != contract.requested_objects
        || receipt.missing_objects > 0
        || receipt.unrecoverable_objects > 0
        || (receipt.independently_accepted_bytes > 0
            && receipt.time_to_first_accepted_byte_ms.is_none());

    let disposition = if required_evidence_incomplete || service_incomplete {
        ServiceDisposition::Incomplete
    } else {
        ServiceDisposition::Pass
    };

    Ok(ServiceScore {
        disposition,
        accepted_bytes_per_second,
        deadline_met,
        missing_required_stages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> WorkloadContract {
        WorkloadContract {
            workload_id: "PSW-TEST-001".into(),
            requested_bytes: 1_000_000,
            requested_objects: 10,
            deadline_ms: 10_000,
            deadline_required: true,
            manifest_required: true,
            provenance_required: true,
            migration_required: false,
            energy_metering_required: true,
            required_energy_boundary: Some(EnergyBoundary::WholeService),
            independent_reader_paths_required: true,
            required_stages: vec![
                Stage::RequestAdmission,
                Stage::MediaMount,
                Stage::ReaderBootstrap,
                Stage::PhysicalRead,
                Stage::DecodeAndReconstruct,
                Stage::FixityAndManifest,
                Stage::ProvenanceAcceptance,
            ],
        }
    }

    fn passing_receipt() -> ServiceReceipt {
        ServiceReceipt {
            workload_id: "PSW-TEST-001".into(),
            elapsed_ms: 5_000,
            time_to_first_accepted_byte_ms: Some(100),
            reconstructed_bytes: 1_000_000,
            delivered_bytes: 1_000_000,
            independently_accepted_bytes: 1_000_000,
            independently_accepted_objects: 10,
            missing_objects: 0,
            extra_objects: 0,
            duplicate_objects: 0,
            unrecoverable_objects: 0,
            false_acceptances: 0,
            expected_digest_status: EvidenceStatus::Pass,
            manifest_status: EvidenceStatus::Pass,
            provenance_status: EvidenceStatus::Pass,
            source_digest_status: EvidenceStatus::NotRequired,
            destination_digest_status: EvidenceStatus::NotRequired,
            migration_lineage_status: EvidenceStatus::NotRequired,
            reader_path_ids: vec!["reader-path-a".into(), "reader-path-b".into()],
            reader_independence_status: EvidenceStatus::Pass,
            reader_agreement_status: EvidenceStatus::Pass,
            energy_meter_status: EvidenceStatus::Pass,
            energy_millijoules: Some(12_345),
            energy_meter_id: Some("meter-whole-service-A".into()),
            energy_calibration_ref: Some("calibration-record-2026-01".into()),
            energy_evidence_sha256: Some("b".repeat(64)),
            energy_boundary: Some(EnergyBoundary::WholeService),
            energy_sample_interval_ms: Some(1_000),
            candidate_claimed_full_success: true,
            stage_observations: contract()
                .required_stages
                .into_iter()
                .map(|stage| StageObservation {
                    stage,
                    elapsed_ms: 100,
                    evidence_present: true,
                })
                .collect(),
        }
    }

    fn migration_contract() -> WorkloadContract {
        let mut value = contract();
        value.migration_required = true;
        value.required_stages.extend([
            Stage::MigrationWrite,
            Stage::MigrationReadback,
            Stage::DestinationVerify,
        ]);
        value
    }

    fn migration_receipt() -> ServiceReceipt {
        let mut value = passing_receipt();
        value.source_digest_status = EvidenceStatus::Pass;
        value.destination_digest_status = EvidenceStatus::Pass;
        value.migration_lineage_status = EvidenceStatus::Pass;
        value.stage_observations.extend(
            [
                Stage::MigrationWrite,
                Stage::MigrationReadback,
                Stage::DestinationVerify,
            ]
            .into_iter()
            .map(|stage| StageObservation {
                stage,
                elapsed_ms: 250,
                evidence_present: true,
            }),
        );
        value
    }

    #[test]
    fn migration_requires_source_destination_and_lineage_evidence() {
        assert_eq!(
            score(&migration_contract(), &migration_receipt())
                .unwrap()
                .disposition,
            ServiceDisposition::Pass
        );

        let mut source_bad = migration_receipt();
        source_bad.source_digest_status = EvidenceStatus::Fail;
        assert_eq!(
            score(&migration_contract(), &source_bad)
                .unwrap()
                .disposition,
            ServiceDisposition::Fail
        );

        let mut destination_bad = migration_receipt();
        destination_bad.destination_digest_status = EvidenceStatus::Fail;
        assert_eq!(
            score(&migration_contract(), &destination_bad)
                .unwrap()
                .disposition,
            ServiceDisposition::Fail
        );

        let mut lineage_missing = migration_receipt();
        lineage_missing.migration_lineage_status = EvidenceStatus::Incomplete;
        lineage_missing.candidate_claimed_full_success = false;
        assert_eq!(
            score(&migration_contract(), &lineage_missing)
                .unwrap()
                .disposition,
            ServiceDisposition::Incomplete
        );
    }

    #[test]
    fn migration_contract_cannot_omit_destination_or_lineage_stages() {
        let mut incomplete_contract = contract();
        incomplete_contract.migration_required = true;
        assert_eq!(
            score(&incomplete_contract, &passing_receipt()),
            Err(ContractError::MissingMigrationStage)
        );
    }

    #[test]
    fn migration_requires_source_read_fixity_and_provenance_stages() {
        let mut no_source_read = migration_contract();
        no_source_read
            .required_stages
            .retain(|stage| *stage != Stage::PhysicalRead);
        assert_eq!(
            score(&no_source_read, &migration_receipt()),
            Err(ContractError::MissingMigrationStage)
        );

        let mut no_provenance = migration_contract();
        no_provenance.provenance_required = false;
        assert_eq!(
            score(&no_provenance, &migration_receipt()),
            Err(ContractError::MigrationRequiresProvenance)
        );
    }

    #[test]
    fn exact_independently_verified_service_passes() {
        let score = score(&contract(), &passing_receipt()).unwrap();
        assert_eq!(score.disposition, ServiceDisposition::Pass);
        assert_eq!(score.accepted_bytes_per_second, 200_000);
        assert_eq!(score.deadline_met, Some(true));
        assert!(score.missing_required_stages.is_empty());
    }

    #[test]
    fn candidate_claim_cannot_override_incomplete_exact_recovery() {
        let mut receipt = passing_receipt();
        receipt.independently_accepted_bytes = 900_000;
        receipt.delivered_bytes = 900_000;
        receipt.reconstructed_bytes = 900_000;
        receipt.independently_accepted_objects = 9;
        receipt.missing_objects = 1;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Fail
        );
    }

    #[test]
    fn a_correct_abstention_remains_incomplete_not_false_acceptance() {
        let mut receipt = passing_receipt();
        receipt.candidate_claimed_full_success = false;
        receipt.independently_accepted_bytes = 900_000;
        receipt.delivered_bytes = 900_000;
        receipt.reconstructed_bytes = 900_000;
        receipt.independently_accepted_objects = 9;
        receipt.missing_objects = 1;
        receipt.expected_digest_status = EvidenceStatus::Incomplete;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Incomplete
        );
    }

    #[test]
    fn digest_manifest_and_provenance_failures_are_hard_failures() {
        for field in 0..3 {
            let mut receipt = passing_receipt();
            match field {
                0 => receipt.expected_digest_status = EvidenceStatus::Fail,
                1 => receipt.manifest_status = EvidenceStatus::Fail,
                _ => receipt.provenance_status = EvidenceStatus::Fail,
            }
            assert_eq!(
                score(&contract(), &receipt).unwrap().disposition,
                ServiceDisposition::Fail
            );
        }
    }

    #[test]
    fn missing_stage_evidence_is_incomplete_unless_candidate_claims_full_success() {
        let mut receipt = passing_receipt();
        receipt
            .stage_observations
            .retain(|item| item.stage != Stage::MediaMount);
        let scored = score(&contract(), &receipt).unwrap();
        assert_eq!(scored.disposition, ServiceDisposition::Fail);
        assert_eq!(scored.missing_required_stages, vec![Stage::MediaMount]);

        receipt.candidate_claimed_full_success = false;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Incomplete
        );
    }

    #[test]
    fn missed_deadline_fails_even_when_all_bytes_are_exact() {
        let mut receipt = passing_receipt();
        receipt.elapsed_ms = 10_001;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Fail
        );
    }

    #[test]
    fn false_acceptance_extra_and_duplicate_objects_fail() {
        for field in 0..3 {
            let mut receipt = passing_receipt();
            match field {
                0 => receipt.false_acceptances = 1,
                1 => receipt.extra_objects = 1,
                _ => receipt.duplicate_objects = 1,
            }
            assert_eq!(
                score(&contract(), &receipt).unwrap().disposition,
                ServiceDisposition::Fail
            );
        }
    }

    #[test]
    fn energy_pass_requires_meter_calibration_boundary_and_interval() {
        let mut missing_meter = passing_receipt();
        missing_meter.energy_meter_id = None;
        assert_eq!(
            score(&contract(), &missing_meter),
            Err(ContractError::MissingEnergyMeterMetadata)
        );

        let mut wrong_boundary = passing_receipt();
        wrong_boundary.energy_boundary = Some(EnergyBoundary::StorageSubsystem);
        assert_eq!(
            score(&contract(), &wrong_boundary),
            Err(ContractError::EnergyMeterBoundaryMismatch)
        );

        let mut zero_interval = passing_receipt();
        zero_interval.energy_sample_interval_ms = Some(0);
        assert_eq!(
            score(&contract(), &zero_interval),
            Err(ContractError::ZeroEnergySampleInterval)
        );

        let mut uncalibrated = passing_receipt();
        uncalibrated.energy_calibration_ref = Some("".into());
        assert_eq!(
            score(&contract(), &uncalibrated),
            Err(ContractError::MissingEnergyMeterMetadata)
        );
    }

    #[test]
    fn energy_pass_requires_a_valid_meter_evidence_digest() {
        let mut missing_boundary = contract();
        missing_boundary.required_energy_boundary = None;
        assert_eq!(
            score(&missing_boundary, &passing_receipt()),
            Err(ContractError::MissingEnergyBoundaryContract)
        );

        let mut malformed_trace = passing_receipt();
        malformed_trace.energy_evidence_sha256 = Some("not-a-digest".into());
        assert_eq!(
            score(&contract(), &malformed_trace),
            Err(ContractError::InvalidEnergyEvidenceDigest)
        );
    }

    #[test]
    // Incomplete required metering cannot support a full-success claim; honest abstention is Incomplete.
fn missing_energy_measurement_requires_honest_abstention() {
        let mut receipt = passing_receipt();
        receipt.energy_meter_status = EvidenceStatus::Incomplete;
        receipt.energy_millijoules = None;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Fail
        );

        receipt.candidate_claimed_full_success = false;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Incomplete
        );
    }

    #[test]
    fn reader_independence_and_agreement_are_separate_hard_gates() {
        let mut disagreement = passing_receipt();
        disagreement.reader_agreement_status = EvidenceStatus::Fail;
        assert_eq!(
            score(&contract(), &disagreement).unwrap().disposition,
            ServiceDisposition::Fail
        );

        let mut shared_dependencies = passing_receipt();
        shared_dependencies.reader_independence_status = EvidenceStatus::Incomplete;
        shared_dependencies.candidate_claimed_full_success = false;
        assert_eq!(
            score(&contract(), &shared_dependencies)
                .unwrap()
                .disposition,
            ServiceDisposition::Incomplete
        );

        let mut one_path = passing_receipt();
        one_path.reader_path_ids.pop();
        one_path.candidate_claimed_full_success = false;
        assert_eq!(
            score(&contract(), &one_path).unwrap().disposition,
            ServiceDisposition::Incomplete
        );
    }

    #[test]
    fn unrecoverable_object_blocks_success() {
        let mut receipt = passing_receipt();
        receipt.unrecoverable_objects = 1;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Fail
        );
        receipt.candidate_claimed_full_success = false;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Incomplete
        );
    }

    #[test]
    fn no_run_is_not_evaluated() {
        let mut receipt = passing_receipt();
        receipt.expected_digest_status = EvidenceStatus::NotEvaluated;
        receipt.manifest_status = EvidenceStatus::NotEvaluated;
        receipt.provenance_status = EvidenceStatus::NotEvaluated;
        receipt.source_digest_status = EvidenceStatus::NotEvaluated;
        receipt.destination_digest_status = EvidenceStatus::NotEvaluated;
        receipt.migration_lineage_status = EvidenceStatus::NotEvaluated;
        receipt.reader_independence_status = EvidenceStatus::NotEvaluated;
        receipt.reader_agreement_status = EvidenceStatus::NotEvaluated;
        receipt.reader_path_ids.clear();
        receipt.energy_meter_status = EvidenceStatus::NotEvaluated;
        receipt.energy_millijoules = None;
        receipt.energy_meter_id = None;
        receipt.energy_calibration_ref = None;
        receipt.energy_evidence_sha256 = None;
        receipt.energy_boundary = None;
        receipt.energy_sample_interval_ms = None;
        receipt.stage_observations.clear();
        receipt.reconstructed_bytes = 0;
        receipt.delivered_bytes = 0;
        receipt.independently_accepted_bytes = 0;
        receipt.independently_accepted_objects = 0;
        receipt.missing_objects = 10;
        receipt.candidate_claimed_full_success = false;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::NotEvaluated
        );
    }

    #[test]
    fn duplicate_stage_and_inconsistent_accounting_are_rejected() {
        let mut duplicate = passing_receipt();
        duplicate
            .stage_observations
            .push(duplicate.stage_observations[0].clone());
        assert_eq!(
            score(&contract(), &duplicate),
            Err(ContractError::ObservedStageDuplicated)
        );

        let mut mismatch = passing_receipt();
        mismatch.missing_objects = 1;
        assert_eq!(
            score(&contract(), &mismatch),
            Err(ContractError::ObjectAccountingMismatch)
        );
    }

    #[test]
    fn delivered_or_accepted_bytes_cannot_exceed_prior_stage_bytes() {
        let mut delivered = passing_receipt();
        delivered.delivered_bytes += 1;
        assert_eq!(
            score(&contract(), &delivered),
            Err(ContractError::DeliveredBytesExceedReconstructed)
        );

        let mut accepted = passing_receipt();
        accepted.independently_accepted_bytes += 1;
        accepted.delivered_bytes += 1;
        accepted.reconstructed_bytes += 1;
        assert_eq!(
            score(&contract(), &accepted),
            Err(ContractError::AcceptedBytesExceedRequested)
        );
    }

    #[test]
    fn missing_first_accepted_byte_time_blocks_a_full_success_claim() {
        let mut receipt = passing_receipt();
        receipt.time_to_first_accepted_byte_ms = None;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Fail
        );

        receipt.candidate_claimed_full_success = false;
        assert_eq!(
            score(&contract(), &receipt).unwrap().disposition,
            ServiceDisposition::Incomplete
        );
    }

    #[test]
    fn a_stage_duration_cannot_exceed_total_wall_clock_time() {
        let mut receipt = passing_receipt();
        receipt.stage_observations[0].elapsed_ms = receipt.elapsed_ms + 1;
        assert_eq!(
            score(&contract(), &receipt),
            Err(ContractError::StageDurationExceedsElapsed)
        );
    }

    #[test]
    fn malformed_reader_paths_and_empty_stage_contract_are_rejected() {
        let mut duplicate_paths = passing_receipt();
        duplicate_paths.reader_path_ids[1] = duplicate_paths.reader_path_ids[0].clone();
        assert_eq!(
            score(&contract(), &duplicate_paths),
            Err(ContractError::DuplicateReaderPathId)
        );

        let mut empty_stages = contract();
        empty_stages.required_stages.clear();
        assert_eq!(
            score(&empty_stages, &passing_receipt()),
            Err(ContractError::NoRequiredStages)
        );
    }

    #[test]
    fn stage_and_time_receipts_must_be_well_formed() {
        let mut first_byte = passing_receipt();
        first_byte.time_to_first_accepted_byte_ms = Some(5_001);
        assert_eq!(
            score(&contract(), &first_byte),
            Err(ContractError::TimeToFirstByteAfterCompletion)
        );

        let mut no_energy = passing_receipt();
        no_energy.energy_millijoules = None;
        assert_eq!(
            score(&contract(), &no_energy),
            Err(ContractError::MeterPassWithoutEnergyValue)
        );
    }
}
