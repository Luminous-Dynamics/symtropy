//! Domain-bound reliability admission for active navigation evidence actions.
//!
//! This module validates externally computed probability intervals for the
//! event "one predeclared acquisition protocol yields a fresh, valid,
//! independently qualified observation in the requested domain". It does not
//! estimate the probability, authenticate source bytes, prove block
//! independence, predict the observation payload, or update navigation state.
//!
//! Every source-justified campaign block contributes at most one Bernoulli
//! outcome to the calibration cohort. Failed, timed-out, invalid, stale and
//! unresolved outcomes are never counted as valid successes. Unresolved
//! outcomes remain visible and are counted conservatively as non-successes.
//!
//! This is an admission gate for a separately reviewed probability interval,
//! not a confidence-bound implementation and not a protection-level guarantee.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const SHA256_BYTES: usize = 32;

/// Outcome of the predeclared acquisition protocol in one independent
/// campaign block. There is one outcome per block; multiple retries within a
/// block do not create additional independent calibration trials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionCampaignOutcome {
    ValidQualifiedSuccess,
    FailedOrTimedOut,
    AcquiredButInvalid,
    UnknownOrUnreconciled,
}

/// One source-justified independent campaign unit in the action-reliability
/// cohort. A unique ID is a bookkeeping constraint, not proof of independence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcquisitionCampaignBlock {
    pub block_id: u64,
    pub independence_group_id: u64,
    pub outcome: AcquisitionCampaignOutcome,
}

/// Externally computed interval for the probability of a valid, qualified
/// acquisition under one frozen action protocol and exact domain.
///
/// The caller must supply a method whose assumptions and numerical result are
/// independently reviewable. This module checks structure and admission
/// constraints; it does not recompute the confidence interval.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidAcquisitionProbabilityContract {
    pub action_id: u64,
    pub domain_fingerprint_sha256: [u8; SHA256_BYTES],
    pub acquisition_protocol_sha256: [u8; SHA256_BYTES],
    pub source_manifest_sha256: [u8; SHA256_BYTES],
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    pub bound_method_sha256: [u8; SHA256_BYTES],
    /// Probability that the reported interval fails to cover the target
    /// valid-acquisition probability under the declared statistical method.
    pub confidence_error_probability: f64,
    pub probability_lower_bound: f64,
    pub probability_upper_bound: f64,
    pub campaign_blocks: Vec<AcquisitionCampaignBlock>,
}

/// Frozen requirements for admitting action reliability in one deployment
/// domain. No cross-domain or cross-protocol pooling is performed.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveEvidenceReliabilityPolicy {
    pub expected_domain_fingerprint_sha256: [u8; SHA256_BYTES],
    pub expected_acquisition_protocol_sha256: [u8; SHA256_BYTES],
    pub minimum_probability_lower_bound: f64,
    pub maximum_confidence_error_probability: f64,
    pub minimum_independent_campaign_blocks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliabilityRejectionReason {
    MissingContract,
    UnsupportedDomain,
    AcquisitionProtocolMismatch,
    InsufficientIndependentCampaignBlocks,
    ConfidenceLevelTooWeak,
    ProbabilityLowerBoundTooSmall,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdmittedAcquisitionReliability {
    pub action_id: u64,
    pub probability_lower_bound: f64,
    pub probability_upper_bound: f64,
    pub confidence_error_probability: f64,
    pub independent_campaign_block_count: usize,
    pub valid_qualified_success_block_count: usize,
    pub non_success_block_count: usize,
    pub unresolved_block_count: usize,
    pub empirical_valid_success_fraction: f64,
    pub source_manifest_sha256: [u8; SHA256_BYTES],
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    pub bound_method_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RejectedAcquisitionReliability {
    pub action_id: u64,
    pub reason: ReliabilityRejectionReason,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveEvidenceReliabilityAdmission {
    pub admitted_actions: Vec<AdmittedAcquisitionReliability>,
    pub rejected_actions: Vec<RejectedAcquisitionReliability>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveEvidenceReliabilityError {
    InvalidPolicy,
    InvalidActionIdentity,
    DuplicateActionId(u64),
    UnexpectedReliabilityContract(u64),
    DuplicateReliabilityContract(u64),
    MissingReliabilityCommitment(u64),
    InvalidProbabilityInterval(u64),
    EmptyCampaignBlocks(u64),
    InvalidCampaignBlockIdentity(u64),
    DuplicateCampaignBlockId(u64),
    DuplicateCampaignIndependenceGroup(u64),
    ArithmeticOverflow(u64),
}

impl fmt::Display for ActiveEvidenceReliabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(f, "reliability policy has invalid probability, confidence, block-count or digest requirements"),
            Self::InvalidActionIdentity => write!(f, "candidate action identity must be nonzero"),
            Self::DuplicateActionId(id) => write!(f, "candidate action ID is duplicated: {id}"),
            Self::UnexpectedReliabilityContract(id) => write!(f, "reliability contract has no matching candidate action: {id}"),
            Self::DuplicateReliabilityContract(id) => write!(f, "reliability contract is duplicated for action: {id}"),
            Self::MissingReliabilityCommitment(id) => write!(f, "reliability contract for action {id} is missing a source, cohort, method, domain or protocol commitment"),
            Self::InvalidProbabilityInterval(id) => write!(f, "reliability interval or empirical success fraction is invalid for action {id}"),
            Self::EmptyCampaignBlocks(id) => write!(f, "reliability contract for action {id} has no independent campaign blocks"),
            Self::InvalidCampaignBlockIdentity(id) => write!(f, "campaign block {id} has a zero block or independence-group identity"),
            Self::DuplicateCampaignBlockId(id) => write!(f, "campaign block ID is duplicated: {id}"),
            Self::DuplicateCampaignIndependenceGroup(id) => write!(f, "independence group is counted more than once: {id}"),
            Self::ArithmeticOverflow(id) => write!(f, "campaign outcome count overflowed for action {id}"),
        }
    }
}

impl std::error::Error for ActiveEvidenceReliabilityError {}

/// Validate reliability contracts for a frozen candidate action set.
///
/// Missing or insufficient contracts are explicit rejections, never implicit
/// success. Every supplied interval must enclose its observed block success
/// fraction. The interval itself remains an external statistical claim and
/// must be verified against its committed method and source cohort.
pub fn admit_active_evidence_reliability(
    policy: &ActiveEvidenceReliabilityPolicy,
    candidate_action_ids: &[u64],
    contracts: &[ValidAcquisitionProbabilityContract],
) -> Result<ActiveEvidenceReliabilityAdmission, ActiveEvidenceReliabilityError> {
    validate_policy(policy)?;

    let mut candidates = BTreeSet::new();
    for action_id in candidate_action_ids {
        if *action_id == 0 {
            return Err(ActiveEvidenceReliabilityError::InvalidActionIdentity);
        }
        if !candidates.insert(*action_id) {
            return Err(ActiveEvidenceReliabilityError::DuplicateActionId(*action_id));
        }
    }

    let mut by_action = BTreeMap::new();
    for contract in contracts {
        if !candidates.contains(&contract.action_id) {
            return Err(ActiveEvidenceReliabilityError::UnexpectedReliabilityContract(
                contract.action_id,
            ));
        }
        if by_action.insert(contract.action_id, contract).is_some() {
            return Err(ActiveEvidenceReliabilityError::DuplicateReliabilityContract(
                contract.action_id,
            ));
        }
    }

    let mut admission = ActiveEvidenceReliabilityAdmission {
        admitted_actions: Vec::new(),
        rejected_actions: Vec::new(),
    };

    for action_id in candidate_action_ids {
        let Some(contract) = by_action.get(action_id).copied() else {
            admission.rejected_actions.push(RejectedAcquisitionReliability {
                action_id: *action_id,
                reason: ReliabilityRejectionReason::MissingContract,
            });
            continue;
        };

        validate_contract_structure(contract)?;

        let reason = if contract.domain_fingerprint_sha256
            != policy.expected_domain_fingerprint_sha256
        {
            Some(ReliabilityRejectionReason::UnsupportedDomain)
        } else if contract.acquisition_protocol_sha256
            != policy.expected_acquisition_protocol_sha256
        {
            Some(ReliabilityRejectionReason::AcquisitionProtocolMismatch)
        } else if contract.campaign_blocks.len()
            < policy.minimum_independent_campaign_blocks
        {
            Some(ReliabilityRejectionReason::InsufficientIndependentCampaignBlocks)
        } else if contract.confidence_error_probability
            > policy.maximum_confidence_error_probability
        {
            Some(ReliabilityRejectionReason::ConfidenceLevelTooWeak)
        } else if contract.probability_lower_bound
            < policy.minimum_probability_lower_bound
        {
            Some(ReliabilityRejectionReason::ProbabilityLowerBoundTooSmall)
        } else {
            None
        };

        if let Some(reason) = reason {
            admission.rejected_actions.push(RejectedAcquisitionReliability {
                action_id: *action_id,
                reason,
            });
            continue;
        }

        let mut successes = 0_usize;
        let mut unresolved = 0_usize;
        for block in &contract.campaign_blocks {
            match block.outcome {
                AcquisitionCampaignOutcome::ValidQualifiedSuccess => {
                    successes = successes.checked_add(1).ok_or(
                        ActiveEvidenceReliabilityError::ArithmeticOverflow(*action_id),
                    )?;
                }
                AcquisitionCampaignOutcome::UnknownOrUnreconciled => {
                    unresolved = unresolved.checked_add(1).ok_or(
                        ActiveEvidenceReliabilityError::ArithmeticOverflow(*action_id),
                    )?;
                }
                AcquisitionCampaignOutcome::FailedOrTimedOut
                | AcquisitionCampaignOutcome::AcquiredButInvalid => {}
            }
        }
        let block_count = contract.campaign_blocks.len();
        let non_successes = block_count.checked_sub(successes).ok_or(
            ActiveEvidenceReliabilityError::ArithmeticOverflow(*action_id),
        )?;
        let empirical_fraction = successes as f64 / block_count as f64;
        if !empirical_fraction.is_finite() {
            return Err(ActiveEvidenceReliabilityError::ArithmeticOverflow(*action_id));
        }

        admission.admitted_actions.push(AdmittedAcquisitionReliability {
            action_id: *action_id,
            probability_lower_bound: contract.probability_lower_bound,
            probability_upper_bound: contract.probability_upper_bound,
            confidence_error_probability: contract.confidence_error_probability,
            independent_campaign_block_count: block_count,
            valid_qualified_success_block_count: successes,
            non_success_block_count: non_successes,
            unresolved_block_count: unresolved,
            empirical_valid_success_fraction: empirical_fraction,
            source_manifest_sha256: contract.source_manifest_sha256,
            calibration_cohort_sha256: contract.calibration_cohort_sha256,
            bound_method_sha256: contract.bound_method_sha256,
        });
    }

    Ok(admission)
}

fn validate_policy(
    policy: &ActiveEvidenceReliabilityPolicy,
) -> Result<(), ActiveEvidenceReliabilityError> {
    if policy.expected_domain_fingerprint_sha256.iter().all(|byte| *byte == 0)
        || policy.expected_acquisition_protocol_sha256.iter().all(|byte| *byte == 0)
        || !policy.minimum_probability_lower_bound.is_finite()
        || !(0.0..=1.0).contains(&policy.minimum_probability_lower_bound)
        || !policy.maximum_confidence_error_probability.is_finite()
        || policy.maximum_confidence_error_probability <= 0.0
        || policy.maximum_confidence_error_probability >= 1.0
        || policy.minimum_independent_campaign_blocks == 0
    {
        return Err(ActiveEvidenceReliabilityError::InvalidPolicy);
    }
    Ok(())
}

fn validate_contract_structure(
    contract: &ValidAcquisitionProbabilityContract,
) -> Result<(), ActiveEvidenceReliabilityError> {
    let action_id = contract.action_id;
    if action_id == 0
        || contract.domain_fingerprint_sha256.iter().all(|byte| *byte == 0)
        || contract.acquisition_protocol_sha256.iter().all(|byte| *byte == 0)
        || contract.source_manifest_sha256.iter().all(|byte| *byte == 0)
        || contract.calibration_cohort_sha256.iter().all(|byte| *byte == 0)
        || contract.bound_method_sha256.iter().all(|byte| *byte == 0)
    {
        return Err(ActiveEvidenceReliabilityError::MissingReliabilityCommitment(
            action_id,
        ));
    }
    if !contract.confidence_error_probability.is_finite()
        || contract.confidence_error_probability <= 0.0
        || contract.confidence_error_probability >= 1.0
        || !contract.probability_lower_bound.is_finite()
        || !contract.probability_upper_bound.is_finite()
        || contract.probability_lower_bound < 0.0
        || contract.probability_upper_bound > 1.0
        || contract.probability_lower_bound > contract.probability_upper_bound
    {
        return Err(ActiveEvidenceReliabilityError::InvalidProbabilityInterval(
            action_id,
        ));
    }
    if contract.campaign_blocks.is_empty() {
        return Err(ActiveEvidenceReliabilityError::EmptyCampaignBlocks(action_id));
    }

    let mut block_ids = BTreeSet::new();
    let mut independence_groups = BTreeSet::new();
    let mut successes = 0_usize;
    for block in &contract.campaign_blocks {
        if block.block_id == 0 || block.independence_group_id == 0 {
            return Err(ActiveEvidenceReliabilityError::InvalidCampaignBlockIdentity(
                block.block_id,
            ));
        }
        if !block_ids.insert(block.block_id) {
            return Err(ActiveEvidenceReliabilityError::DuplicateCampaignBlockId(
                block.block_id,
            ));
        }
        if !independence_groups.insert(block.independence_group_id) {
            return Err(
                ActiveEvidenceReliabilityError::DuplicateCampaignIndependenceGroup(
                    block.independence_group_id,
                ),
            );
        }
        if block.outcome == AcquisitionCampaignOutcome::ValidQualifiedSuccess {
            successes = successes.checked_add(1).ok_or(
                ActiveEvidenceReliabilityError::ArithmeticOverflow(action_id),
            )?;
        }
    }
    let empirical_fraction = successes as f64 / contract.campaign_blocks.len() as f64;
    if !empirical_fraction.is_finite()
        || contract.probability_lower_bound > empirical_fraction
        || contract.probability_upper_bound < empirical_fraction
    {
        return Err(ActiveEvidenceReliabilityError::InvalidProbabilityInterval(
            action_id,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ActiveEvidenceReliabilityPolicy {
        ActiveEvidenceReliabilityPolicy {
            expected_domain_fingerprint_sha256: [0x11; SHA256_BYTES],
            expected_acquisition_protocol_sha256: [0x22; SHA256_BYTES],
            minimum_probability_lower_bound: 0.7,
            maximum_confidence_error_probability: 0.05,
            minimum_independent_campaign_blocks: 4,
        }
    }

    fn contract(action_id: u64) -> ValidAcquisitionProbabilityContract {
        ValidAcquisitionProbabilityContract {
            action_id,
            domain_fingerprint_sha256: [0x11; SHA256_BYTES],
            acquisition_protocol_sha256: [0x22; SHA256_BYTES],
            source_manifest_sha256: [0x33; SHA256_BYTES],
            calibration_cohort_sha256: [0x44; SHA256_BYTES],
            bound_method_sha256: [0x55; SHA256_BYTES],
            confidence_error_probability: 0.05,
            probability_lower_bound: 0.75,
            probability_upper_bound: 1.0,
            campaign_blocks: vec![
                AcquisitionCampaignBlock { block_id: 1, independence_group_id: 101, outcome: AcquisitionCampaignOutcome::ValidQualifiedSuccess },
                AcquisitionCampaignBlock { block_id: 2, independence_group_id: 102, outcome: AcquisitionCampaignOutcome::ValidQualifiedSuccess },
                AcquisitionCampaignBlock { block_id: 3, independence_group_id: 103, outcome: AcquisitionCampaignOutcome::ValidQualifiedSuccess },
                AcquisitionCampaignBlock { block_id: 4, independence_group_id: 104, outcome: AcquisitionCampaignOutcome::ValidQualifiedSuccess },
            ],
        }
    }

    #[test]
    fn admits_only_a_domain_and_protocol_matched_interval_with_sufficient_blocks() {
        let result = admit_active_evidence_reliability(&policy(), &[7], &[contract(7)]).unwrap();
        assert_eq!(result.admitted_actions.len(), 1);
        assert!(result.rejected_actions.is_empty());
        let admitted = result.admitted_actions[0];
        assert_eq!(admitted.action_id, 7);
        assert_eq!(admitted.independent_campaign_block_count, 4);
        assert_eq!(admitted.valid_qualified_success_block_count, 4);
        assert_eq!(admitted.non_success_block_count, 0);
        assert_eq!(admitted.empirical_valid_success_fraction, 1.0);
    }

    #[test]
    fn missing_contract_is_an_explicit_rejection_not_an_assumed_success() {
        let result = admit_active_evidence_reliability(&policy(), &[7], &[]).unwrap();
        assert!(result.admitted_actions.is_empty());
        assert_eq!(
            result.rejected_actions,
            vec![RejectedAcquisitionReliability {
                action_id: 7,
                reason: ReliabilityRejectionReason::MissingContract,
            }]
        );
    }

    #[test]
    fn domain_and_protocol_mismatches_are_separate_rejections() {
        let mut other_domain = contract(7);
        other_domain.domain_fingerprint_sha256 = [0x66; SHA256_BYTES];
        let mut other_protocol = contract(8);
        other_protocol.acquisition_protocol_sha256 = [0x77; SHA256_BYTES];
        let result = admit_active_evidence_reliability(
            &policy(),
            &[7, 8],
            &[other_domain, other_protocol],
        )
        .unwrap();
        assert_eq!(
            result.rejected_actions,
            vec![
                RejectedAcquisitionReliability {
                    action_id: 7,
                    reason: ReliabilityRejectionReason::UnsupportedDomain,
                },
                RejectedAcquisitionReliability {
                    action_id: 8,
                    reason: ReliabilityRejectionReason::AcquisitionProtocolMismatch,
                },
            ]
        );
    }

    #[test]
    fn unresolved_campaign_outcomes_are_counted_as_non_successes() {
        let mut candidate = contract(7);
        candidate.probability_lower_bound = 0.5;
        candidate.probability_upper_bound = 1.0;
        candidate.campaign_blocks[3].outcome =
            AcquisitionCampaignOutcome::UnknownOrUnreconciled;
        let mut cfg = policy();
        cfg.minimum_probability_lower_bound = 0.5;
        let result = admit_active_evidence_reliability(&cfg, &[7], &[candidate]).unwrap();
        let admitted = result.admitted_actions[0];
        assert_eq!(admitted.valid_qualified_success_block_count, 3);
        assert_eq!(admitted.non_success_block_count, 1);
        assert_eq!(admitted.unresolved_block_count, 1);
        assert_eq!(admitted.empirical_valid_success_fraction, 0.75);
    }

    #[test]
    fn duplicate_campaign_or_independence_group_ids_are_rejected() {
        let mut candidate = contract(7);
        candidate.campaign_blocks[1].independence_group_id =
            candidate.campaign_blocks[0].independence_group_id;
        assert_eq!(
            admit_active_evidence_reliability(&policy(), &[7], &[candidate]),
            Err(ActiveEvidenceReliabilityError::DuplicateCampaignIndependenceGroup(101))
        );
    }

    #[test]
    fn intervals_must_enclose_the_observed_valid_success_fraction() {
        let mut candidate = contract(7);
        candidate.probability_lower_bound = 0.9;
        candidate.probability_upper_bound = 1.0;
        assert_eq!(
            admit_active_evidence_reliability(&policy(), &[7], &[candidate]),
            Err(ActiveEvidenceReliabilityError::InvalidProbabilityInterval(7))
        );
    }

    #[test]
    fn confidence_and_lower_bound_requirements_are_fail_closed() {
        let mut weak_confidence = contract(7);
        weak_confidence.confidence_error_probability = 0.1;
        let mut low_bound = contract(8);
        low_bound.probability_lower_bound = 0.6;
        let result = admit_active_evidence_reliability(
            &policy(),
            &[7, 8],
            &[weak_confidence, low_bound],
        )
        .unwrap();
        assert_eq!(
            result.rejected_actions,
            vec![
                RejectedAcquisitionReliability {
                    action_id: 7,
                    reason: ReliabilityRejectionReason::ConfidenceLevelTooWeak,
                },
                RejectedAcquisitionReliability {
                    action_id: 8,
                    reason: ReliabilityRejectionReason::ProbabilityLowerBoundTooSmall,
                },
            ]
        );
    }

    #[test]
    fn malformed_policy_or_duplicate_actions_are_rejected() {
        let mut invalid = policy();
        invalid.expected_domain_fingerprint_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            admit_active_evidence_reliability(&invalid, &[7], &[]),
            Err(ActiveEvidenceReliabilityError::InvalidPolicy)
        );
        assert_eq!(
            admit_active_evidence_reliability(&policy(), &[7, 7], &[]),
            Err(ActiveEvidenceReliabilityError::DuplicateActionId(7))
        );
    }
}
