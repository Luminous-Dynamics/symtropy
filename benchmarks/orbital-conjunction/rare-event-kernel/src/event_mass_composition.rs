// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fail-closed composition of admitted event mass and independent residual mass.
//!
//! The estimator input is an unnormalised event-mass contribution on the
//! admitted region, not P(event | admitted region). The residual input bounds
//! event mass on the disjoint complement under the same target distribution.
//!
//! The envelope is [admitted_lower, min(1, admitted_upper + residual_upper)].
//! This is valid only if the admitted and residual regions form a disjoint,
//! exhaustive partition. The caller supplies that partition declaration;
//! this module checks identity consistency but cannot prove the geometry.

use crate::KernelError;
use crate::residual::AffineResidualMassReport;

/// Immutable event and target-distribution identity shared by source evidence.
///
/// The event-definition digest must commit to event semantics including horizon,
/// time scale, frame, hard-body geometry and event convention. The target digest
/// commits to the exact nominal probability measure and physical-model inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventModelIdentity {
    /// Stable conjunction/event identifier.
    pub event_id: String,
    /// Immutable event-definition revision.
    pub event_revision: String,
    /// SHA-256 digest of the complete event definition and horizon semantics.
    pub event_definition_sha256: String,
    /// Immutable physical-model revision.
    pub model_revision: String,
    /// SHA-256 digest of the nominal target probability distribution.
    pub target_distribution_sha256: String,
}

/// What the partition declaration asserts about the admitted and residual regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionClaim {
    /// Caller asserts that the regions are disjoint and cover the complete
    /// declared event domain under the target distribution.
    CallerDeclaredDisjointAndExhaustive,
    /// No disjoint/exhaustive partition claim has been established.
    Unverified,
}

/// Immutable evidence identifying the partition used by both source reports.
///
/// This is a caller-supplied declaration. The kernel checks identity agreement
/// but does not independently verify the geometric or measure-theoretic claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMassPartitionDeclaration {
    /// Event/model identity for the domain being partitioned.
    pub identity: EventModelIdentity,
    /// Stable identifier for this partition declaration.
    pub declaration_id: String,
    /// SHA-256 digest of the partition definition and region membership.
    pub partition_sha256: String,
    /// Region whose event mass is estimated by the sampler.
    pub admitted_region_id: String,
    /// Complementary region whose event mass is bounded independently.
    pub residual_region_id: String,
    /// SHA-256 digest of the declaration/proof artifact.
    pub declaration_sha256: String,
    /// Explicit caller claim about partition disjointness and exhaustiveness.
    pub claim: PartitionClaim,
}

/// Upstream interval for the unnormalised event mass inside the admitted region.
///
/// This is not a probability conditional on being in the admitted region. It
/// must bound the admitted region's contribution to the target event probability.
#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedEventMassEvidence {
    /// Event/model identity for this evidence.
    pub identity: EventModelIdentity,
    /// SHA-256 digest of the partition to which this estimate applies.
    pub partition_sha256: String,
    /// Region identifier; must equal the declaration's admitted region.
    pub region_id: String,
    /// Stable identifier of the upstream estimator result.
    pub evidence_id: String,
    /// SHA-256 digest of the immutable upstream result artifact.
    pub source_sha256: String,
    /// Lower bound on target event mass inside the admitted region, in [0, 1].
    pub event_mass_lower: f64,
    /// Upper bound on target event mass inside the admitted region, in [0, 1].
    pub event_mass_upper: f64,
}

/// Whether the residual-mass search fully accounted for the declared complement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualMassCoverage {
    /// The residual verifier accounts for the complete declared complement
    /// under the named target distribution.
    Complete,
    /// Some residual domain or event mass remains outside the certificate.
    Incomplete,
    /// The residual computation budget was exhausted before complete accounting.
    BudgetExhausted,
}

/// Independent upper bound on target event mass in the residual region.
#[derive(Debug, Clone, PartialEq)]
pub struct ResidualEventMassEvidence {
    /// Event/model identity for this evidence.
    pub identity: EventModelIdentity,
    /// SHA-256 digest of the partition to which this residual bound applies.
    pub partition_sha256: String,
    /// Region identifier; must equal the declaration's residual region.
    pub region_id: String,
    /// Stable identifier of the independent residual verifier result.
    pub evidence_id: String,
    /// SHA-256 digest of the immutable residual-certificate artifact.
    pub source_sha256: String,
    /// Conservative upper bound on residual event mass, in [0, 1].
    pub residual_event_mass_upper: f64,
    /// Whether the complete declared residual region has been accounted for.
    pub coverage: ResidualMassCoverage,
}

/// Basis on which the composed envelope is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventMassCompositionBasis {
    /// Valid conditional on the caller's disjoint/exhaustive partition claim
    /// and the validity of both upstream evidence items.
    ConditionalOnCallerDeclaredPartition,
}

/// Full event-mass envelope after matching admitted and residual evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposedEventMassEnvelope {
    /// Shared immutable event/model identity.
    pub identity: EventModelIdentity,
    /// SHA-256 digest of the exact partition declaration.
    pub partition_sha256: String,
    /// Identifier of the partition declaration artifact.
    pub partition_declaration_id: String,
    /// Identifier of the admitted event region.
    pub admitted_region_id: String,
    /// Identifier of the residual complement region.
    pub residual_region_id: String,
    /// Original estimator evidence identifier.
    pub admitted_evidence_id: String,
    /// Original residual-certificate evidence identifier.
    pub residual_evidence_id: String,
    /// Original admitted-region event-mass interval, preserved unchanged.
    pub admitted_event_mass_interval: (f64, f64),
    /// Original residual event-mass upper bound, preserved unchanged.
    pub residual_event_mass_upper: f64,
    /// Lower bound on the complete target event probability.
    pub full_event_probability_lower: f64,
    /// Conservative upper bound on the complete target event probability.
    pub full_event_probability_upper: f64,
    /// Explicit basis and assumptions of the composition.
    pub basis: EventMassCompositionBasis,
}

/// Construct residual evidence from the existing static affine-union verifier report.
///
/// A non-budget-exhausted report is marked Complete even when it contains
/// unresolved cells: the verifier includes a conservative mass upper bound
/// for every unresolved cell and the probability outside its finite cube.
/// A budget-exhausted report is kept explicitly unqualified by this adapter.
///
/// The caller must bind the report to the correct event/model identity, region,
/// partition and immutable source artifact. This helper validates the scalar
/// bound and carries the verifier's budget state; it cannot prove that those
/// caller-supplied identities actually describe the affine model.
pub fn residual_event_mass_evidence_from_affine_union_report(
    identity: EventModelIdentity,
    partition_sha256: String,
    region_id: String,
    evidence_id: String,
    source_sha256: String,
    report: &AffineResidualMassReport,
) -> Result<ResidualEventMassEvidence, KernelError> {
    validate_identity(&identity)?;
    validate_digest(&partition_sha256)?;
    validate_identifier(&region_id)?;
    validate_identifier(&evidence_id)?;
    validate_digest(&source_sha256)?;
    validate_probability(report.residual_event_mass_upper_bound)?;

    let coverage = if report.budget_exhausted {
        ResidualMassCoverage::BudgetExhausted
    } else {
        ResidualMassCoverage::Complete
    };

    Ok(ResidualEventMassEvidence {
        identity,
        partition_sha256,
        region_id,
        evidence_id,
        source_sha256,
        residual_event_mass_upper: report.residual_event_mass_upper_bound,
        coverage,
    })
}

/// Compose admitted-region event mass with an independent residual upper bound.
///
/// The composition is rejected unless all event/model/target/partition identities
/// agree, region IDs match a caller-declared disjoint/exhaustive partition, both
/// source artifacts have valid SHA-256 digest syntax, and the residual verifier
/// marks the entire complement as accounted for.
///
/// This result is conditional on the upstream estimator, residual verifier and
/// caller partition declaration being correct. It is not a proof of coverage,
/// physical-model adequacy or estimator validity.
pub fn compose_admitted_and_residual_event_mass(
    admitted: &AdmittedEventMassEvidence,
    residual: &ResidualEventMassEvidence,
    partition: &EventMassPartitionDeclaration,
) -> Result<ComposedEventMassEnvelope, KernelError> {
    validate_identity(&admitted.identity)?;
    validate_identity(&residual.identity)?;
    if admitted.identity != residual.identity {
        return Err(KernelError::EvidenceIdentityMismatch);
    }

    validate_identity(&partition.identity)?;
    if partition.identity != admitted.identity {
        return Err(KernelError::EvidenceIdentityMismatch);
    }
    validate_identifier(&partition.declaration_id)?;
    validate_digest(&partition.partition_sha256)?;
    validate_digest(&partition.declaration_sha256)?;
    validate_identifier(&partition.admitted_region_id)?;
    validate_identifier(&partition.residual_region_id)?;
    if partition.admitted_region_id == partition.residual_region_id {
        return Err(KernelError::IdenticalEvidenceRegions);
    }
    if partition.claim != PartitionClaim::CallerDeclaredDisjointAndExhaustive {
        return Err(KernelError::InvalidPartitionDeclaration);
    }

    validate_identifier(&admitted.evidence_id)?;
    validate_digest(&admitted.source_sha256)?;
    validate_identifier(&admitted.region_id)?;
    validate_probability_interval(admitted.event_mass_lower, admitted.event_mass_upper)?;

    validate_identifier(&residual.evidence_id)?;
    validate_digest(&residual.source_sha256)?;
    validate_identifier(&residual.region_id)?;
    validate_probability(residual.residual_event_mass_upper)?;

    if admitted.partition_sha256 != partition.partition_sha256
        || residual.partition_sha256 != partition.partition_sha256
    {
        return Err(KernelError::PartitionIdentityMismatch);
    }
    if admitted.region_id != partition.admitted_region_id
        || residual.region_id != partition.residual_region_id
    {
        return Err(KernelError::PartitionIdentityMismatch);
    }
    if admitted.evidence_id == residual.evidence_id
        || admitted.source_sha256 == residual.source_sha256
    {
        return Err(KernelError::InvalidEvidenceIdentifier);
    }
    match residual.coverage {
        ResidualMassCoverage::Complete => {}
        ResidualMassCoverage::Incomplete => {
            return Err(KernelError::ResidualEvidenceIncomplete);
        }
        ResidualMassCoverage::BudgetExhausted => {
            return Err(KernelError::ResidualEvidenceBudgetExhausted);
        }
    }

    let raw_upper = admitted.event_mass_upper + residual.residual_event_mass_upper;
    if !raw_upper.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let full_event_probability_upper = if raw_upper >= 1.0 {
        1.0
    } else if raw_upper == 0.0 {
        0.0
    } else {
        next_up(raw_upper).min(1.0)
    };

    Ok(ComposedEventMassEnvelope {
        identity: admitted.identity.clone(),
        partition_sha256: partition.partition_sha256.clone(),
        partition_declaration_id: partition.declaration_id.clone(),
        admitted_region_id: admitted.region_id.clone(),
        residual_region_id: residual.region_id.clone(),
        admitted_evidence_id: admitted.evidence_id.clone(),
        residual_evidence_id: residual.evidence_id.clone(),
        admitted_event_mass_interval: (admitted.event_mass_lower, admitted.event_mass_upper),
        residual_event_mass_upper: residual.residual_event_mass_upper,
        full_event_probability_lower: admitted.event_mass_lower,
        full_event_probability_upper,
        basis: EventMassCompositionBasis::ConditionalOnCallerDeclaredPartition,
    })
}

fn validate_identity(identity: &EventModelIdentity) -> Result<(), KernelError> {
    validate_identifier(&identity.event_id)?;
    validate_identifier(&identity.event_revision)?;
    validate_digest(&identity.event_definition_sha256)?;
    validate_identifier(&identity.model_revision)?;
    validate_digest(&identity.target_distribution_sha256)
}

fn validate_identifier(value: &str) -> Result<(), KernelError> {
    if value.trim().is_empty() {
        Err(KernelError::InvalidEvidenceIdentifier)
    } else {
        Ok(())
    }
}

fn validate_digest(value: &str) -> Result<(), KernelError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Err(KernelError::InvalidEvidenceDigest)
    } else {
        Ok(())
    }
}

fn validate_probability(value: f64) -> Result<(), KernelError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        Err(KernelError::InvalidProbability)
    } else {
        Ok(())
    }
}

fn validate_probability_interval(lower: f64, upper: f64) -> Result<(), KernelError> {
    validate_probability(lower)?;
    validate_probability(upper)?;
    if lower > upper {
        Err(KernelError::InvalidProbabilityInterval)
    } else {
        Ok(())
    }
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HASH_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn identity() -> EventModelIdentity {
        EventModelIdentity {
            event_id: "event-42".to_owned(),
            event_revision: "event-r7".to_owned(),
            event_definition_sha256: HASH_A.to_owned(),
            model_revision: "force-model-r3".to_owned(),
            target_distribution_sha256: HASH_B.to_owned(),
        }
    }

    fn admitted() -> AdmittedEventMassEvidence {
        AdmittedEventMassEvidence {
            identity: identity(),
            partition_sha256: HASH_C.to_owned(),
            region_id: "admitted-union".to_owned(),
            evidence_id: "is-run-183".to_owned(),
            source_sha256: HASH_D.to_owned(),
            event_mass_lower: 1.0e-6,
            event_mass_upper: 1.2e-6,
        }
    }

    fn residual() -> ResidualEventMassEvidence {
        ResidualEventMassEvidence {
            identity: identity(),
            partition_sha256: HASH_C.to_owned(),
            region_id: "residual-complement".to_owned(),
            evidence_id: "residual-run-52".to_owned(),
            source_sha256: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                .to_owned(),
            residual_event_mass_upper: 0.2e-6,
            coverage: ResidualMassCoverage::Complete,
        }
    }

    fn partition() -> EventMassPartitionDeclaration {
        EventMassPartitionDeclaration {
            identity: identity(),
            declaration_id: "partition-r9".to_owned(),
            partition_sha256: HASH_C.to_owned(),
            admitted_region_id: "admitted-union".to_owned(),
            residual_region_id: "residual-complement".to_owned(),
            declaration_sha256: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                .to_owned(),
            claim: PartitionClaim::CallerDeclaredDisjointAndExhaustive,
        }
    }

    fn affine_residual_report(
        residual_upper: f64,
        unresolved_cells: usize,
        budget_exhausted: bool,
    ) -> AffineResidualMassReport {
        AffineResidualMassReport {
            visited_cells: 16,
            chart_covered_cells: 3,
            event_free_cells: 8,
            unresolved_cells,
            budget_exhausted,
            residual_event_mass_upper_bound: residual_upper,
        }
    }

    #[test]
    fn affine_union_report_adapter_preserves_unresolved_mass_as_a_complete_bound() {
        let report = affine_residual_report(2.5e-7, 4, false);
        let evidence = residual_event_mass_evidence_from_affine_union_report(
            identity(),
            HASH_C.to_owned(),
            "residual-complement".to_owned(),
            "affine-union-run-53".to_owned(),
            "1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
            &report,
        )
        .expect("conservative affine union report");
        assert_eq!(evidence.coverage, ResidualMassCoverage::Complete);
        assert_eq!(evidence.residual_event_mass_upper, 2.5e-7);
    }

    #[test]
    fn affine_union_report_adapter_preserves_budget_exhaustion() {
        let report = affine_residual_report(1.0, 9, true);
        let evidence = residual_event_mass_evidence_from_affine_union_report(
            identity(),
            HASH_C.to_owned(),
            "residual-complement".to_owned(),
            "affine-union-run-54".to_owned(),
            "2222222222222222222222222222222222222222222222222222222222222222".to_owned(),
            &report,
        )
        .expect("budget-exhausted report is still recorded");
        assert_eq!(evidence.coverage, ResidualMassCoverage::BudgetExhausted);
    }

    #[test]
    fn affine_union_report_adapter_rejects_invalid_mass_or_identity() {
        let invalid_mass = affine_residual_report(f64::NAN, 0, false);
        assert_eq!(
            residual_event_mass_evidence_from_affine_union_report(
                identity(),
                HASH_C.to_owned(),
                "residual-complement".to_owned(),
                "affine-run-invalid".to_owned(),
                "1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
                &invalid_mass,
            ),
            Err(KernelError::InvalidProbability)
        );

        let valid = affine_residual_report(0.01, 0, false);
        assert_eq!(
            residual_event_mass_evidence_from_affine_union_report(
                identity(),
                "bad-partition-digest".to_owned(),
                "residual-complement".to_owned(),
                "affine-run-invalid".to_owned(),
                "1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
                &valid,
            ),
            Err(KernelError::InvalidEvidenceDigest)
        );
    }

    #[test]
    fn matching_evidence_composes_only_the_full_event_upper_bound() {
        let result =
            compose_admitted_and_residual_event_mass(&admitted(), &residual(), &partition())
                .expect("matching evidence");
        assert_eq!(result.admitted_event_mass_interval, (1.0e-6, 1.2e-6));
        assert_eq!(result.residual_event_mass_upper, 0.2e-6);
        assert_eq!(result.full_event_probability_lower, 1.0e-6);
        assert!(result.full_event_probability_upper >= 1.4e-6);
        assert_eq!(
            result.basis,
            EventMassCompositionBasis::ConditionalOnCallerDeclaredPartition
        );
    }

    #[test]
    fn upper_bound_is_capped_at_one() {
        let mut admitted = admitted();
        admitted.event_mass_upper = 0.8;
        let mut residual = residual();
        residual.residual_event_mass_upper = 0.4;
        let result = compose_admitted_and_residual_event_mass(&admitted, &residual, &partition())
            .expect("matching high mass evidence");
        assert_eq!(result.full_event_probability_upper, 1.0);
    }

    #[test]
    fn event_and_model_identity_mismatches_are_rejected() {
        let mut residual_input = residual();
        residual_input.identity.event_revision = "event-r8".to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::EvidenceIdentityMismatch)
        );

        let mut residual_input = residual();
        residual_input.identity.model_revision = "force-model-r4".to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::EvidenceIdentityMismatch)
        );

        let mut residual_input = residual();
        residual_input.identity.target_distribution_sha256 = HASH_D.to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::EvidenceIdentityMismatch)
        );
    }

    #[test]
    fn partition_event_model_identity_mismatch_is_rejected() {
        let mut partition = partition();
        partition.identity.model_revision = "different-force-model".to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual(), &partition),
            Err(KernelError::EvidenceIdentityMismatch)
        );
    }

    #[test]
    fn event_definition_digest_mismatch_is_rejected() {
        let mut residual_input = residual();
        residual_input.identity.event_definition_sha256 = HASH_D.to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::EvidenceIdentityMismatch)
        );
    }

    #[test]
    fn partition_declaration_must_match_the_same_event_and_target_model() {
        let mut partition_input = partition();
        partition_input.identity.target_distribution_sha256 = HASH_D.to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual(), &partition_input),
            Err(KernelError::EvidenceIdentityMismatch)
        );
    }

    #[test]
    fn partition_digest_mismatch_is_rejected() {
        let mut residual_input = residual();
        residual_input.partition_sha256 = HASH_D.to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::PartitionIdentityMismatch)
        );

        let mut partition = partition();
        partition.partition_sha256 = HASH_D.to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual(), &partition),
            Err(KernelError::PartitionIdentityMismatch)
        );
    }

    #[test]
    fn wrong_region_assignment_and_identical_regions_are_rejected() {
        let mut residual_input = residual();
        residual_input.region_id = "admitted-union".to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::PartitionIdentityMismatch)
        );

        let mut partition = partition();
        partition.residual_region_id = "admitted-union".to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual(), &partition),
            Err(KernelError::IdenticalEvidenceRegions)
        );
    }

    #[test]
    fn incomplete_or_budget_exhausted_residual_evidence_is_rejected() {
        let mut residual_input = residual();
        residual_input.coverage = ResidualMassCoverage::Incomplete;
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::ResidualEvidenceIncomplete)
        );

        let mut residual_input = residual();
        residual_input.coverage = ResidualMassCoverage::BudgetExhausted;
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::ResidualEvidenceBudgetExhausted)
        );
    }

    #[test]
    fn unverified_partition_is_rejected() {
        let mut partition = partition();
        partition.claim = PartitionClaim::Unverified;
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual(), &partition),
            Err(KernelError::InvalidPartitionDeclaration)
        );
    }

    #[test]
    fn malformed_digests_and_blank_ids_are_rejected() {
        let mut admitted_input = admitted();
        admitted_input.source_sha256 = "not-a-hash".to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted_input, &residual(), &partition()),
            Err(KernelError::InvalidEvidenceDigest)
        );

        let mut partition = partition();
        partition.declaration_id = " ".to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual(), &partition),
            Err(KernelError::InvalidEvidenceIdentifier)
        );
    }

    #[test]
    fn invalid_probability_bounds_fail_closed() {
        let mut admitted_input = admitted();
        admitted_input.event_mass_lower = 0.4;
        admitted_input.event_mass_upper = 0.3;
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted_input, &residual(), &partition()),
            Err(KernelError::InvalidProbabilityInterval)
        );

        let mut residual_input = residual();
        residual_input.residual_event_mass_upper = 1.1;
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::InvalidProbability)
        );
    }

    #[test]
    fn distinct_upstream_evidence_artifacts_are_required() {
        let mut residual_input = residual();
        residual_input.evidence_id = "is-run-183".to_owned();
        residual_input.source_sha256 = HASH_D.to_owned();
        assert_eq!(
            compose_admitted_and_residual_event_mass(&admitted(), &residual_input, &partition()),
            Err(KernelError::InvalidEvidenceIdentifier)
        );
    }
}
