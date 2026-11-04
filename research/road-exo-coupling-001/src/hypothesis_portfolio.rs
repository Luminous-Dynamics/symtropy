//! Open-world hypothesis portfolio, challenge coverage and residual-risk assurance.
//!
//! A finite hypothesis set is never presumed complete relative to reality.
//! This module records its declared coverage, common-mode lineage, challenge
//! results and residual utility envelope. "No counterexample found" means only
//! that a bounded challenge run did not find one.

use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;
use crate::decision_closure::{DecisionError, DecisionManifest, ExactRational};

const DOMAIN: &[u8] = b"symtropy.road-hypothesis-portfolio.v0.1\0";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PortfolioClaimScope {
    EmpiricalRoadDecision,
    SimulationChallenge,
    FormalConformance,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CoverageDimension {
    OddAttributeAndCombination,
    BehaviouralCompetency,
    CausalMechanism,
    RuleAndSafety,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisLineageRecord {
    pub hypothesis_id: StableId,
    pub hypothesis_revision_digest: [u8; 32],
    /// Declared common-mode roots: source logs, simulator families, pretraining,
    /// code ancestry, shared parameter sources or other shared generating roots.
    pub common_mode_root_digests: BTreeSet<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageObligation {
    pub obligation_id: StableId,
    pub dimension: CoverageDimension,
    pub scope_id: StableId,
    pub disposition: CoverageDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoverageDisposition {
    Covered {
        evidence_digest: [u8; 32],
        protocol_digest: [u8; 32],
        independent_unit_count: u64,
        challenge_run_ids: BTreeSet<StableId>,
    },
    NotCovered {
        reason_id: StableId,
    },
    Unknown {
        reason_id: StableId,
    },
    NotApplicable {
        rationale_id: StableId,
    },
    OutOfScope {
        reason_id: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeRun {
    pub challenge_id: StableId,
    pub protocol_digest: [u8; 32],
    pub holdout_split_digest: [u8; 32],
    pub blinded: bool,
    pub independent_unit_count: u64,
    /// Search budget is a declared, versioned unit (e.g. simulator steps,
    /// wall-clock milliseconds, or evaluation queries).
    pub budget_units: u64,
    pub coverage_dimensions: BTreeSet<CoverageDimension>,
    pub result: ChallengeResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChallengeResult {
    CounterexampleFound {
        counterexample_id: StableId,
        counterexample_digest: [u8; 32],
        new_mechanism_id: StableId,
    },
    NoCounterexampleWithinBudget,
    Incomplete {
        reason_id: StableId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidualEvidenceOrigin {
    IndependentObservedCalibration,
    ControlledInterventionCalibration,
    QualifiedNaturalExperiment,
    FormalAnalyticBound,
    SimulationOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualActionBound {
    pub action_id: StableId,
    pub utility_lower: ExactRational,
    pub utility_upper: ExactRational,
    pub feasibility: ResidualFeasibility,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidualFeasibility {
    GuaranteedFeasible,
    PotentialViolation {
        reason_id: StableId,
    },
    Unknown {
        reason_id: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidualEnvelope {
    Bounded {
        origin: ResidualEvidenceOrigin,
        evidence_digest: [u8; 32],
        calibration_digest: [u8; 32],
        independent_unit_count: u64,
        bounds: Vec<ResidualActionBound>,
    },
    Unbounded {
        reason_id: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisPortfolioManifest {
    pub portfolio_id: StableId,
    pub revision: u64,
    pub parent_portfolio_digest: Option<[u8; 32]>,
    pub claim_scope: PortfolioClaimScope,
    pub decision: DecisionManifest,
    /// Required dimensions are explicit so an entire missing dimension cannot
    /// disappear from the coverage denominator.
    pub required_dimensions: BTreeSet<CoverageDimension>,
    pub hypotheses: Vec<HypothesisLineageRecord>,
    pub coverage_obligations: Vec<CoverageObligation>,
    pub challenge_runs: Vec<ChallengeRun>,
    pub residual_envelope: ResidualEnvelope,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortfolioStatus {
    PortfolioCompleteForDeclaredScope,
    PortfolioIncomplete,
    PortfolioFalsified,
    OutOfScope,
    ResidualRiskUnbounded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageDimensionSummary {
    pub dimension: CoverageDimension,
    pub required: usize,
    pub covered: usize,
    pub not_covered: usize,
    pub unknown: usize,
    pub not_applicable: usize,
    pub out_of_scope: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommonModeGroup {
    /// Stable representative: lexicographically smallest hypothesis ID.
    pub representative: StableId,
    pub members: Vec<StableId>,
    pub shared_root_digests: BTreeSet<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredCounterexample {
    pub challenge_id: StableId,
    pub counterexample_id: StableId,
    pub counterexample_digest: [u8; 32],
    pub new_mechanism_id: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualRiskSummary {
    pub bounded: bool,
    pub empirical_origin_eligible: bool,
    pub guaranteed_feasible_actions: Vec<StableId>,
    pub excluded_actions: Vec<StableId>,
    pub envelope_width_by_action: Vec<(StableId, ExactRational)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisPortfolioCertificate {
    pub schema_id: &'static str,
    pub manifest_digest: [u8; 32],
    pub status: PortfolioStatus,
    pub coverage: Vec<CoverageDimensionSummary>,
    pub common_mode_groups: Vec<CommonModeGroup>,
    pub declared_hypothesis_count: usize,
    pub effective_common_mode_family_count: usize,
    pub challenge_run_count: usize,
    pub no_counterexample_run_count: usize,
    pub discovered_counterexamples: Vec<DiscoveredCounterexample>,
    pub residual_risk: ResidualRiskSummary,
    pub claim_ceiling: StableId,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortfolioError {
    DecisionInvalid(DecisionError),
    DecisionNotFullySupported,
    RevisionMustBePositive,
    MissingParentPortfolioDigest,
    UnexpectedParentPortfolioDigest,
    DuplicateHypothesis,
    HypothesisLineageMismatch,
    MissingHypothesisLineage,
    EmptyCommonModeRoots,
    ZeroHypothesisRevisionDigest,
    DuplicateCoverageObligation,
    EmptyCoverageManifest,
    InvalidCoveredEvidence,
    MissingCoverageChallenge,
    UnknownCoverageChallenge,
    DuplicateChallenge,
    InvalidChallengeProtocol,
    InvalidChallengeHoldout,
    ChallengeNotBlinded,
    EmptyChallengeDimensions,
    MissingChallengeUnits,
    MissingChallengeBudget,
    InvalidCounterexample,
    DuplicateCoverageReference,
    MissingRequiredCoverageDimension,
    UnexpectedCoverageDimension,
    ChallengeCoverageMismatch,
    UnknownAction,
    DuplicateResidualAction,
    MissingResidualAction,
    InvalidResidualDigest,
    MissingResidualUnits,
    InvalidResidualInterval,
    WrongResidualEvidenceOrigin,
    ZeroDenominator,
    ArithmeticOverflow,
    ReceiptMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortfolioMutationError {
    NotAnAppendOnlyRevision,
    PortfolioIdMismatch,
    ParentDigestMismatch,
}

impl HypothesisPortfolioManifest {
    pub fn validate(&self) -> Result<(), PortfolioError> {
        self.decision
            .validate()
            .map_err(PortfolioError::DecisionInvalid)?;
        let decision = self
            .decision
            .evaluate()
            .map_err(PortfolioError::DecisionInvalid)?;
        if matches!(
            decision.status,
            crate::decision_closure::DecisionStatus::Unsupported
                | crate::decision_closure::DecisionStatus::Incomplete
        ) {
            return Err(PortfolioError::DecisionNotFullySupported);
        }

        if self.revision == 0 {
            return Err(PortfolioError::RevisionMustBePositive);
        }
        if self.revision == 1 && self.parent_portfolio_digest.is_some() {
            return Err(PortfolioError::UnexpectedParentPortfolioDigest);
        }
        if self.revision > 1 && self.parent_portfolio_digest.is_none() {
            return Err(PortfolioError::MissingParentPortfolioDigest);
        }

        let decision_hypotheses = self
            .decision
            .hypotheses
            .iter()
            .map(|hypothesis| hypothesis.hypothesis_id.clone())
            .collect::<BTreeSet<_>>();
        let mut lineage_ids = BTreeSet::new();
        for lineage in &self.hypotheses {
            if !lineage_ids.insert(lineage.hypothesis_id.clone()) {
                return Err(PortfolioError::DuplicateHypothesis);
            }
            if lineage.hypothesis_revision_digest == [0; 32] {
                return Err(PortfolioError::ZeroHypothesisRevisionDigest);
            }
            if lineage.common_mode_root_digests.is_empty() {
                return Err(PortfolioError::EmptyCommonModeRoots);
            }
            if lineage.common_mode_root_digests.contains(&[0; 32]) {
                return Err(PortfolioError::EmptyCommonModeRoots);
            }
        }
        if lineage_ids != decision_hypotheses {
            return Err(PortfolioError::HypothesisLineageMismatch);
        }
        if self.hypotheses.is_empty() {
            return Err(PortfolioError::MissingHypothesisLineage);
        }

        if self.coverage_obligations.is_empty() {
            return Err(PortfolioError::EmptyCoverageManifest);
        }
        if self.required_dimensions.is_empty() {
            return Err(PortfolioError::MissingRequiredCoverageDimension);
        }
        let declared_dimensions = self
            .coverage_obligations
            .iter()
            .map(|obligation| obligation.dimension)
            .collect::<BTreeSet<_>>();
        if !self.required_dimensions.is_subset(&declared_dimensions) {
            return Err(PortfolioError::MissingRequiredCoverageDimension);
        }
        if !declared_dimensions.is_subset(&self.required_dimensions) {
            return Err(PortfolioError::UnexpectedCoverageDimension);
        }

        let mut obligation_ids = BTreeSet::new();
        let mut challenge_ids = BTreeSet::new();
        for run in &self.challenge_runs {
            if !challenge_ids.insert(run.challenge_id.clone()) {
                return Err(PortfolioError::DuplicateChallenge);
            }
            if run.protocol_digest == [0; 32] {
                return Err(PortfolioError::InvalidChallengeProtocol);
            }
            if run.holdout_split_digest == [0; 32] {
                return Err(PortfolioError::InvalidChallengeHoldout);
            }
            if !run.blinded {
                return Err(PortfolioError::ChallengeNotBlinded);
            }
            if run.coverage_dimensions.is_empty() {
                return Err(PortfolioError::EmptyChallengeDimensions);
            }
            if run.independent_unit_count == 0 {
                return Err(PortfolioError::MissingChallengeUnits);
            }
            if run.budget_units == 0 {
                return Err(PortfolioError::MissingChallengeBudget);
            }
            if let ChallengeResult::CounterexampleFound {
                counterexample_digest,
                ..
            } = &run.result
                && *counterexample_digest == [0; 32]
            {
                return Err(PortfolioError::InvalidCounterexample);
            }
        }

        for obligation in &self.coverage_obligations {
            if !obligation_ids.insert(obligation.obligation_id.clone()) {
                return Err(PortfolioError::DuplicateCoverageObligation);
            }
            if obligation.scope_id.as_str().is_empty() {
                return Err(PortfolioError::EmptyCoverageManifest);
            }
            match &obligation.disposition {
                CoverageDisposition::Covered {
                    evidence_digest,
                    protocol_digest,
                    independent_unit_count,
                    challenge_run_ids,
                } => {
                    if *evidence_digest == [0; 32]
                        || *protocol_digest == [0; 32]
                        || *independent_unit_count == 0
                    {
                        return Err(PortfolioError::InvalidCoveredEvidence);
                    }
                    if challenge_run_ids.is_empty() {
                        return Err(PortfolioError::MissingCoverageChallenge);
                    }
                    for challenge_id in challenge_run_ids {
                        if !challenge_ids.contains(challenge_id) {
                            return Err(PortfolioError::UnknownCoverageChallenge);
                        }
                        let run = self
                            .challenge_runs
                            .iter()
                            .find(|run| &run.challenge_id == challenge_id)
                            .ok_or(PortfolioError::UnknownCoverageChallenge)?;
                        if !run.coverage_dimensions.contains(&obligation.dimension) {
                            return Err(PortfolioError::ChallengeCoverageMismatch);
                        }
                    }
                }
                CoverageDisposition::NotCovered { reason_id }
                | CoverageDisposition::Unknown { reason_id }
                | CoverageDisposition::NotApplicable {
                    rationale_id: reason_id,
                }
                | CoverageDisposition::OutOfScope { reason_id } => {
                    if reason_id.as_str().is_empty() {
                        return Err(PortfolioError::EmptyCoverageManifest);
                    }
                }
            }
        }

        for run in &self.challenge_runs {
            let referenced = self.coverage_obligations.iter().any(|obligation| {
                matches!(
                    &obligation.disposition,
                    CoverageDisposition::Covered {
                        challenge_run_ids,
                        ..
                    } if challenge_run_ids.contains(&run.challenge_id)
                )
            });
            if !referenced {
                return Err(PortfolioError::DuplicateCoverageReference);
            }
        }

        self.validate_residual_envelope()
    }

    fn validate_residual_envelope(&self) -> Result<(), PortfolioError> {
        let actions = self.decision.actions.iter().cloned().collect::<BTreeSet<_>>();
        match &self.residual_envelope {
            ResidualEnvelope::Unbounded { reason_id } => {
                if reason_id.as_str().is_empty() {
                    return Err(PortfolioError::InvalidResidualDigest);
                }
            }
            ResidualEnvelope::Bounded {
                origin,
                evidence_digest,
                qualification_receipt_digest,
                calibration_digest,
                independent_unit_count,
                bounds,
            } => {
                if *evidence_digest == [0; 32] || *calibration_digest == [0; 32] {
                    return Err(PortfolioError::InvalidResidualDigest);
                }
                if *independent_unit_count == 0 {
                    return Err(PortfolioError::MissingResidualUnits);
                }
                let origin_allowed = match self.claim_scope {
                    PortfolioClaimScope::EmpiricalRoadDecision => matches!(
                        origin,
                        ResidualEvidenceOrigin::IndependentObservedCalibration
                            | ResidualEvidenceOrigin::ControlledInterventionCalibration
                            | ResidualEvidenceOrigin::QualifiedNaturalExperiment
                    ),
                    PortfolioClaimScope::SimulationChallenge => matches!(
                        origin,
                        ResidualEvidenceOrigin::SimulationOnly
                            | ResidualEvidenceOrigin::IndependentObservedCalibration
                            | ResidualEvidenceOrigin::ControlledInterventionCalibration
                            | ResidualEvidenceOrigin::QualifiedNaturalExperiment
                    ),
                    PortfolioClaimScope::FormalConformance => {
                        matches!(origin, ResidualEvidenceOrigin::FormalAnalyticBound)
                    }
                };
                if !origin_allowed {
                    return Err(PortfolioError::WrongResidualEvidenceOrigin);
                }

                let mut seen = BTreeSet::new();
                for bound in bounds {
                    if !actions.contains(&bound.action_id) {
                        return Err(PortfolioError::UnknownAction);
                    }
                    if !seen.insert(bound.action_id.clone()) {
                        return Err(PortfolioError::DuplicateResidualAction);
                    }
                    let lower = normalize(&bound.utility_lower)?;
                    let upper = normalize(&bound.utility_upper)?;
                    if cmp(&lower, &upper)? == std::cmp::Ordering::Greater {
                        return Err(PortfolioError::InvalidResidualInterval);
                    }
                }
                if seen != actions {
                    return Err(PortfolioError::MissingResidualAction);
                }
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PortfolioError> {
        self.validate()?;
        let decision_digest = self
            .decision
            .digest()
            .map_err(PortfolioError::DecisionInvalid)?;

        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.portfolio_id);
        out.extend_from_slice(&self.revision.to_le_bytes());
        match self.parent_portfolio_digest {
            Some(digest) => {
                out.push(1);
                out.extend_from_slice(&digest);
            }
            None => out.push(0),
        }
        out.push(self.claim_scope as u8);
        out.extend_from_slice(&decision_digest);
        out.push(self.required_dimensions.len() as u8);
        for dimension in &self.required_dimensions {
            out.push(*dimension as u8);
        }

        let mut hypotheses = self.hypotheses.iter().collect::<Vec<_>>();
        hypotheses.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
        push_len(&mut out, hypotheses.len());
        for hypothesis in hypotheses {
            push_id(&mut out, &hypothesis.hypothesis_id);
            out.extend_from_slice(&hypothesis.hypothesis_revision_digest);
            push_len(&mut out, hypothesis.common_mode_root_digests.len());
            for digest in &hypothesis.common_mode_root_digests {
                out.extend_from_slice(digest);
            }
        }

        let mut obligations = self.coverage_obligations.iter().collect::<Vec<_>>();
        obligations.sort_by(|a, b| a.obligation_id.cmp(&b.obligation_id));
        push_len(&mut out, obligations.len());
        for obligation in obligations {
            push_id(&mut out, &obligation.obligation_id);
            out.push(obligation.dimension as u8);
            push_id(&mut out, &obligation.scope_id);
            match &obligation.disposition {
                CoverageDisposition::Covered {
                    evidence_digest,
                    protocol_digest,
                    independent_unit_count,
                    challenge_run_ids,
                } => {
                    out.push(1);
                    out.extend_from_slice(evidence_digest);
                    out.extend_from_slice(protocol_digest);
                    out.extend_from_slice(&independent_unit_count.to_le_bytes());
                    push_ids(&mut out, challenge_run_ids.iter());
                }
                CoverageDisposition::NotCovered { reason_id } => {
                    out.push(2);
                    push_id(&mut out, reason_id);
                }
                CoverageDisposition::Unknown { reason_id } => {
                    out.push(3);
                    push_id(&mut out, reason_id);
                }
                CoverageDisposition::NotApplicable { rationale_id } => {
                    out.push(4);
                    push_id(&mut out, rationale_id);
                }
                CoverageDisposition::OutOfScope { reason_id } => {
                    out.push(5);
                    push_id(&mut out, reason_id);
                }
            }
        }

        let mut runs = self.challenge_runs.iter().collect::<Vec<_>>();
        runs.sort_by(|a, b| a.challenge_id.cmp(&b.challenge_id));
        push_len(&mut out, runs.len());
        for run in runs {
            push_id(&mut out, &run.challenge_id);
            out.extend_from_slice(&run.protocol_digest);
            out.extend_from_slice(&run.holdout_split_digest);
            out.push(u8::from(run.blinded));
            out.extend_from_slice(&run.independent_unit_count.to_le_bytes());
            out.extend_from_slice(&run.budget_units.to_le_bytes());
            out.push(run.coverage_dimensions.len() as u8);
            for dimension in &run.coverage_dimensions {
                out.push(*dimension as u8);
            }
            match &run.result {
                ChallengeResult::CounterexampleFound {
                    counterexample_id,
                    counterexample_digest,
                    new_mechanism_id,
                } => {
                    out.push(1);
                    push_id(&mut out, counterexample_id);
                    out.extend_from_slice(counterexample_digest);
                    push_id(&mut out, new_mechanism_id);
                }
                ChallengeResult::NoCounterexampleWithinBudget => out.push(2),
                ChallengeResult::Incomplete { reason_id } => {
                    out.push(3);
                    push_id(&mut out, reason_id);
                }
            }
        }

        push_residual_envelope(&mut out, &self.residual_envelope);
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], PortfolioError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(&self) -> Result<HypothesisPortfolioCertificate, PortfolioError> {
        self.validate()?;
        let manifest_digest = self.digest()?;
        let coverage = coverage_summary(&self.coverage_obligations);
        let common_mode_groups = common_mode_groups(&self.hypotheses);
        let discovered_counterexamples = self
            .challenge_runs
            .iter()
            .filter_map(|run| match &run.result {
                ChallengeResult::CounterexampleFound {
                    counterexample_id,
                    counterexample_digest,
                    new_mechanism_id,
                } => Some(DiscoveredCounterexample {
                    challenge_id: run.challenge_id.clone(),
                    counterexample_id: counterexample_id.clone(),
                    counterexample_digest: *counterexample_digest,
                    new_mechanism_id: new_mechanism_id.clone(),
                }),
                ChallengeResult::NoCounterexampleWithinBudget | ChallengeResult::Incomplete { .. } => None,
            })
            .collect::<Vec<_>>();
        let mut discovered_counterexamples = discovered_counterexamples;
        discovered_counterexamples.sort_by(|a, b| a.challenge_id.cmp(&b.challenge_id));

        let no_counterexample_run_count = self
            .challenge_runs
            .iter()
            .filter(|run| matches!(&run.result, ChallengeResult::NoCounterexampleWithinBudget))
            .count();

        let residual_risk = residual_summary(&self.residual_envelope, &self.decision)?;
        let status = determine_status(
            &self.coverage_obligations,
            &self.challenge_runs,
            &self.residual_envelope,
            self.claim_scope,
        );
        let claim_ceiling = claim_ceiling_id(status, self.claim_scope);

        let mut certificate = HypothesisPortfolioCertificate {
            schema_id: "symtropy.road-hypothesis-portfolio.certificate.v0.1",
            manifest_digest,
            status,
            coverage,
            effective_common_mode_family_count: common_mode_groups.len(),
            declared_hypothesis_count: self.hypotheses.len(),
            common_mode_groups,
            challenge_run_count: self.challenge_runs.len(),
            no_counterexample_run_count,
            discovered_counterexamples,
            residual_risk,
            claim_ceiling,
            receipt_digest: [0; 32],
        };
        certificate.receipt_digest =
            *blake3::hash(&portfolio_certificate_bytes(&certificate)).as_bytes();
        Ok(certificate)
    }

    /// Append-only revision check: the child must point at the exact parent digest.
    pub fn verify_successor_of(
        &self,
        parent: &HypothesisPortfolioManifest,
    ) -> Result<(), PortfolioMutationError> {
        if self.portfolio_id != parent.portfolio_id {
            return Err(PortfolioMutationError::PortfolioIdMismatch);
        }
        if self.revision != parent.revision + 1 {
            return Err(PortfolioMutationError::NotAnAppendOnlyRevision);
        }
        if self.parent_portfolio_digest != Some(
            parent
                .digest()
                .map_err(|_| PortfolioMutationError::ParentDigestMismatch)?,
        ) {
            return Err(PortfolioMutationError::ParentDigestMismatch);
        }
        Ok(())
    }
}

impl HypothesisPortfolioCertificate {
    /// Unkeyed checksum only; exact replay is the verification path.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&portfolio_certificate_bytes(self)).as_bytes() == self.receipt_digest
    }

    pub fn verify_against(
        &self,
        manifest: &HypothesisPortfolioManifest,
    ) -> Result<(), PortfolioError> {
        let recomputed = manifest.evaluate()?;
        if recomputed == *self {
            Ok(())
        } else {
            Err(PortfolioError::ReceiptMismatch)
        }
    }
}

fn determine_status(
    coverage: &[CoverageObligation],
    runs: &[ChallengeRun],
    residual: &ResidualEnvelope,
    claim_scope: PortfolioClaimScope,
) -> PortfolioStatus {
    if runs.iter().any(|run| matches!(&run.result, ChallengeResult::CounterexampleFound { .. })) {
        return PortfolioStatus::PortfolioFalsified;
    }
    if coverage.iter().any(|obligation| matches!(
        &obligation.disposition,
        CoverageDisposition::OutOfScope { .. }
    )) {
        return PortfolioStatus::OutOfScope;
    }
    if coverage.iter().any(|obligation| matches!(
        &obligation.disposition,
        CoverageDisposition::NotCovered { .. } | CoverageDisposition::Unknown { .. }
    )) || runs.iter().any(|run| matches!(&run.result, ChallengeResult::Incomplete { .. }))
        || runs.is_empty()
    {
        return PortfolioStatus::PortfolioIncomplete;
    }

    match residual {
        ResidualEnvelope::Unbounded { .. } => PortfolioStatus::ResidualRiskUnbounded,
        ResidualEnvelope::Bounded { origin, bounds, .. } => {
            let origin_allowed = match claim_scope {
                PortfolioClaimScope::EmpiricalRoadDecision => matches!(
                    origin,
                    ResidualEvidenceOrigin::IndependentObservedCalibration
                        | ResidualEvidenceOrigin::ControlledInterventionCalibration
                        | ResidualEvidenceOrigin::QualifiedNaturalExperiment
                ),
                PortfolioClaimScope::SimulationChallenge => true,
                PortfolioClaimScope::FormalConformance => {
                    matches!(origin, ResidualEvidenceOrigin::FormalAnalyticBound)
                }
            };
            let all_feasible = bounds
                .iter()
                .all(|bound| matches!(&bound.feasibility, ResidualFeasibility::GuaranteedFeasible));
            if !origin_allowed || !all_feasible {
                PortfolioStatus::ResidualRiskUnbounded
            } else if coverage.iter().all(|obligation| matches!(
                &obligation.disposition,
                CoverageDisposition::Covered { .. } | CoverageDisposition::NotApplicable { .. }
            )) && runs.iter().all(|run| matches!(
                &run.result,
                ChallengeResult::NoCounterexampleWithinBudget
            )) {
                PortfolioStatus::PortfolioCompleteForDeclaredScope
            } else {
                PortfolioStatus::PortfolioIncomplete
            }
        }
    }
}

fn coverage_summary(obligations: &[CoverageObligation]) -> Vec<CoverageDimensionSummary> {
    let dimensions = [
        CoverageDimension::OddAttributeAndCombination,
        CoverageDimension::BehaviouralCompetency,
        CoverageDimension::CausalMechanism,
        CoverageDimension::RuleAndSafety,
    ];
    dimensions
        .iter()
        .map(|dimension| {
            let selected = obligations
                .iter()
                .filter(|obligation| obligation.dimension == *dimension)
                .collect::<Vec<_>>();
            let mut summary = CoverageDimensionSummary {
                dimension: *dimension,
                required: selected.len(),
                covered: 0,
                not_covered: 0,
                unknown: 0,
                not_applicable: 0,
                out_of_scope: 0,
            };
            for obligation in selected {
                match &obligation.disposition {
                    CoverageDisposition::Covered { .. } => summary.covered += 1,
                    CoverageDisposition::NotCovered { .. } => summary.not_covered += 1,
                    CoverageDisposition::Unknown { .. } => summary.unknown += 1,
                    CoverageDisposition::NotApplicable { .. } => summary.not_applicable += 1,
                    CoverageDisposition::OutOfScope { .. } => summary.out_of_scope += 1,
                }
            }
            summary
        })
        .collect()
}

fn common_mode_groups(hypotheses: &[HypothesisLineageRecord]) -> Vec<CommonModeGroup> {
    let mut parent = (0..hypotheses.len()).collect::<Vec<_>>();
    let mut root_owner = BTreeMap::<[u8; 32], usize>::new();

    for (index, hypothesis) in hypotheses.iter().enumerate() {
        for root in &hypothesis.common_mode_root_digests {
            if let Some(previous) = root_owner.get(root).copied() {
                union(&mut parent, index, previous);
            } else {
                root_owner.insert(*root, index);
            }
        }
    }

    let mut grouped = BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..hypotheses.len() {
        let representative = find(&mut parent, index);
        grouped.entry(representative).or_default().push(index);
    }

    let mut groups = grouped
        .into_values()
        .map(|members| {
            let mut ids = members
                .iter()
                .map(|index| hypotheses[*index].hypothesis_id.clone())
                .collect::<Vec<_>>();
            ids.sort();
            let roots = members
                .iter()
                .flat_map(|index| hypotheses[*index].common_mode_root_digests.iter().copied())
                .collect::<BTreeSet<_>>();
            CommonModeGroup {
                representative: ids[0].clone(),
                members: ids,
                shared_root_digests: roots,
            }
        })
        .collect::<Vec<_>>();
    groups.sort_by(|a, b| a.representative.cmp(&b.representative));
    groups
}

fn find(parent: &mut [usize], index: usize) -> usize {
    if parent[index] != index {
        let root = find(parent, parent[index]);
        parent[index] = root;
    }
    parent[index]
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find(parent, left);
    let right_root = find(parent, right);
    if left_root != right_root {
        let (lower, higher) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        parent[higher] = lower;
    }
}

fn residual_summary(
    residual: &ResidualEnvelope,
    decision: &DecisionManifest,
) -> Result<ResidualRiskSummary, PortfolioError> {
    match residual {
        ResidualEnvelope::Unbounded { .. } => Ok(ResidualRiskSummary {
            bounded: false,
            empirical_origin_eligible: false,
            guaranteed_feasible_actions: Vec::new(),
            excluded_actions: decision.actions.clone(),
            envelope_width_by_action: Vec::new(),
        }),
        ResidualEnvelope::Bounded { origin, bounds, .. } => {
            let empirical_origin_eligible = matches!(
                origin,
                ResidualEvidenceOrigin::IndependentObservedCalibration
                    | ResidualEvidenceOrigin::ControlledInterventionCalibration
                    | ResidualEvidenceOrigin::QualifiedNaturalExperiment
            );
            let mut guaranteed = Vec::new();
            let mut excluded = Vec::new();
            let mut widths = Vec::new();
            for bound in bounds {
                let lower = normalize(&bound.utility_lower)?;
                let upper = normalize(&bound.utility_upper)?;
                let width = upper
                    .checked_sub(&lower)
                    .map_err(|_| PortfolioError::ArithmeticOverflow)?;
                widths.push((bound.action_id.clone(), width));
                if matches!(bound.feasibility, ResidualFeasibility::GuaranteedFeasible) {
                    guaranteed.push(bound.action_id.clone());
                } else {
                    excluded.push(bound.action_id.clone());
                }
            }
            guaranteed.sort();
            excluded.sort();
            widths.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(ResidualRiskSummary {
                bounded: true,
                empirical_origin_eligible,
                guaranteed_feasible_actions: guaranteed,
                excluded_actions: excluded,
                envelope_width_by_action: widths,
            })
        }
    }
}

fn claim_ceiling_id(status: PortfolioStatus, scope: PortfolioClaimScope) -> StableId {
    let value = match (status, scope) {
        (PortfolioStatus::PortfolioCompleteForDeclaredScope, PortfolioClaimScope::EmpiricalRoadDecision) => "empirical-within-declared-scope-only",
        (PortfolioStatus::PortfolioCompleteForDeclaredScope, PortfolioClaimScope::SimulationChallenge) => "simulation-challenge-only",
        (PortfolioStatus::PortfolioCompleteForDeclaredScope, PortfolioClaimScope::FormalConformance) => "formal-fixture-domain-only",
        (PortfolioStatus::PortfolioFalsified, _) => "portfolio-falsified-no-robust-claim",
        (PortfolioStatus::ResidualRiskUnbounded, _) => "residual-risk-unbounded-no-robust-claim",
        (PortfolioStatus::OutOfScope, _) => "out-of-scope-no-claim",
        (PortfolioStatus::PortfolioIncomplete, _) => "incomplete-portfolio-no-robust-claim",
    };
    StableId::new(value).expect("static claim ceiling ID")
}

fn normalize(value: &ExactRational) -> Result<ExactRational, PortfolioError> {
    ExactRational::new(value.numerator, value.denominator)
        .map_err(|_| PortfolioError::ZeroDenominator)
}

fn cmp(left: &ExactRational, right: &ExactRational) -> Result<std::cmp::Ordering, PortfolioError> {
    left.checked_cmp(right)
        .map_err(|_| PortfolioError::ArithmeticOverflow)
}

fn push_residual_envelope(out: &mut Vec<u8>, residual: &ResidualEnvelope) {
    match residual {
        ResidualEnvelope::Unbounded { reason_id } => {
            out.push(1);
            push_id(out, reason_id);
        }
        ResidualEnvelope::Bounded {
            origin,
            evidence_digest,
            qualification_receipt_digest,
            calibration_digest,
            independent_unit_count,
            bounds,
        } => {
            out.push(2);
            out.push(*origin as u8);
            out.extend_from_slice(evidence_digest);
            out.extend_from_slice(qualification_receipt_digest);
            out.extend_from_slice(calibration_digest);
            out.extend_from_slice(&independent_unit_count.to_le_bytes());
            let mut bounds = bounds.iter().collect::<Vec<_>>();
            bounds.sort_by(|a, b| a.action_id.cmp(&b.action_id));
            push_len(out, bounds.len());
            for bound in bounds {
                push_id(out, &bound.action_id);
                push_rational(out, &bound.utility_lower);
                push_rational(out, &bound.utility_upper);
                match &bound.feasibility {
                    ResidualFeasibility::GuaranteedFeasible => out.push(1),
                    ResidualFeasibility::PotentialViolation { reason_id } => {
                        out.push(2);
                        push_id(out, reason_id);
                    }
                    ResidualFeasibility::Unknown { reason_id } => {
                        out.push(3);
                        push_id(out, reason_id);
                    }
                }
            }
        }
    }
}

fn portfolio_certificate_bytes(certificate: &HypothesisPortfolioCertificate) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-hypothesis-portfolio.certificate.v0.1\0");
    out.extend_from_slice(&certificate.manifest_digest);
    out.push(certificate.status as u8);
    push_len(&mut out, certificate.coverage.len());
    for summary in &certificate.coverage {
        out.push(summary.dimension as u8);
        for value in [
            summary.required,
            summary.covered,
            summary.not_covered,
            summary.unknown,
            summary.not_applicable,
            summary.out_of_scope,
        ] {
            push_len(&mut out, value);
        }
    }
    push_len(&mut out, certificate.common_mode_groups.len());
    for group in &certificate.common_mode_groups {
        push_id(&mut out, &group.representative);
        push_ids(&mut out, group.members.iter());
        push_len(&mut out, group.shared_root_digests.len());
        for digest in &group.shared_root_digests {
            out.extend_from_slice(digest);
        }
    }
    push_len(&mut out, certificate.declared_hypothesis_count);
    push_len(&mut out, certificate.effective_common_mode_family_count);
    push_len(&mut out, certificate.challenge_run_count);
    push_len(&mut out, certificate.no_counterexample_run_count);
    push_len(&mut out, certificate.discovered_counterexamples.len());
    for counterexample in &certificate.discovered_counterexamples {
        push_id(&mut out, &counterexample.challenge_id);
        push_id(&mut out, &counterexample.counterexample_id);
        out.extend_from_slice(&counterexample.counterexample_digest);
        push_id(&mut out, &counterexample.new_mechanism_id);
    }
    out.push(u8::from(certificate.residual_risk.bounded));
    out.push(u8::from(certificate.residual_risk.empirical_origin_eligible));
    push_ids(&mut out, certificate.residual_risk.guaranteed_feasible_actions.iter());
    push_ids(&mut out, certificate.residual_risk.excluded_actions.iter());
    push_len(&mut out, certificate.residual_risk.envelope_width_by_action.len());
    for (action, width) in &certificate.residual_risk.envelope_width_by_action {
        push_id(&mut out, action);
        push_rational(&mut out, width);
    }
    push_id(&mut out, &certificate.claim_ceiling);
    out
}

fn push_ids<'a>(out: &mut Vec<u8>, ids: impl Iterator<Item = &'a StableId>) {
    let values = ids.collect::<Vec<_>>();
    push_len(out, values.len());
    for id in values {
        push_id(out, id);
    }
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_rational(out: &mut Vec<u8>, value: &ExactRational) {
    out.extend_from_slice(&value.numerator.to_le_bytes());
    out.extend_from_slice(&value.denominator.to_le_bytes());
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("collection length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_closure::{DecisionHypothesis, HypothesisActionOutcome, OutcomeSupport};

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn hypothesis(name: &str, root: [u8; 32]) -> HypothesisLineageRecord {
        HypothesisLineageRecord {
            hypothesis_id: id(name),
            hypothesis_revision_digest: [3; 32],
            common_mode_root_digests: BTreeSet::from([root]),
        }
    }

    fn decision() -> DecisionManifest {
        let h1 = DecisionHypothesis {
            hypothesis_id: id("model-a"),
            reference_manifest_digest: [11; 32],
            outcomes: vec![
                HypothesisActionOutcome {
                    action_id: id("action-a"),
                    support: OutcomeSupport::Supported {
                        utility: ExactRational::new(8, 1).unwrap(),
                        evidence_digest: [1; 32],
                    },
                },
                HypothesisActionOutcome {
                    action_id: id("action-b"),
                    support: OutcomeSupport::Supported {
                        utility: ExactRational::new(4, 1).unwrap(),
                        evidence_digest: [2; 32],
                    },
                },
            ],
        };
        let h2 = DecisionHypothesis {
            hypothesis_id: id("model-b"),
            reference_manifest_digest: [12; 32],
            outcomes: vec![
                HypothesisActionOutcome {
                    action_id: id("action-a"),
                    support: OutcomeSupport::Supported {
                        utility: ExactRational::new(6, 1).unwrap(),
                        evidence_digest: [3; 32],
                    },
                },
                HypothesisActionOutcome {
                    action_id: id("action-b"),
                    support: OutcomeSupport::Supported {
                        utility: ExactRational::new(5, 1).unwrap(),
                        evidence_digest: [4; 32],
                    },
                },
            ],
        };
        DecisionManifest {
            decision_id: id("portfolio-decision"),
            version: id("v0.1"),
            utility_scale_id: id("road-safety-utility-v1"),
            utility_semantics_id: id("higher-is-better-v1"),
            actions: vec![id("action-a"), id("action-b")],
            hypotheses: vec![h1, h2],
        }
    }

    fn challenge(result: ChallengeResult) -> ChallengeRun {
        ChallengeRun {
            challenge_id: id("challenge-main"),
            protocol_digest: [5; 32],
            holdout_split_digest: [6; 32],
            blinded: true,
            independent_unit_count: 24,
            budget_units: 10_000,
            coverage_dimensions: BTreeSet::from([
                CoverageDimension::OddAttributeAndCombination,
                CoverageDimension::BehaviouralCompetency,
                CoverageDimension::CausalMechanism,
                CoverageDimension::RuleAndSafety,
            ]),
            result,
        }
    }

    fn coverage(dimension: CoverageDimension) -> CoverageObligation {
        CoverageObligation {
            obligation_id: id(match dimension {
                CoverageDimension::OddAttributeAndCombination => "coverage-odd",
                CoverageDimension::BehaviouralCompetency => "coverage-behaviour",
                CoverageDimension::CausalMechanism => "coverage-causal",
                CoverageDimension::RuleAndSafety => "coverage-rules",
            }),
            dimension,
            scope_id: id("road-crossing-night-rain-occlusion"),
            disposition: CoverageDisposition::Covered {
                evidence_digest: [7; 32],
                protocol_digest: [8; 32],
                independent_unit_count: 24,
                challenge_run_ids: BTreeSet::from([id("challenge-main")]),
            },
        }
    }

    fn residual() -> ResidualEnvelope {
        ResidualEnvelope::Bounded {
            origin: ResidualEvidenceOrigin::IndependentObservedCalibration,
            evidence_digest: [9; 32],
            qualification_receipt_digest: [35; 32],
            calibration_digest: [10; 32],
            independent_unit_count: 40,
            bounds: vec![
                ResidualActionBound {
                    action_id: id("action-a"),
                    utility_lower: ExactRational::new(-2, 1).unwrap(),
                    utility_upper: ExactRational::new(9, 1).unwrap(),
                    feasibility: ResidualFeasibility::GuaranteedFeasible,
                },
                ResidualActionBound {
                    action_id: id("action-b"),
                    utility_lower: ExactRational::new(-1, 1).unwrap(),
                    utility_upper: ExactRational::new(7, 1).unwrap(),
                    feasibility: ResidualFeasibility::GuaranteedFeasible,
                },
            ],
        }
    }

    fn manifest() -> HypothesisPortfolioManifest {
        HypothesisPortfolioManifest {
            portfolio_id: id("road-portfolio-v1"),
            revision: 1,
            parent_portfolio_digest: None,
            claim_scope: PortfolioClaimScope::EmpiricalRoadDecision,
            decision: decision(),
            required_dimensions: BTreeSet::from([
                CoverageDimension::OddAttributeAndCombination,
                CoverageDimension::BehaviouralCompetency,
                CoverageDimension::CausalMechanism,
                CoverageDimension::RuleAndSafety,
            ]),
            hypotheses: vec![hypothesis("model-a", [21; 32]), hypothesis("model-b", [22; 32])],
            coverage_obligations: vec![
                coverage(CoverageDimension::OddAttributeAndCombination),
                coverage(CoverageDimension::BehaviouralCompetency),
                coverage(CoverageDimension::CausalMechanism),
                coverage(CoverageDimension::RuleAndSafety),
            ],
            challenge_runs: vec![challenge(ChallengeResult::NoCounterexampleWithinBudget)],
            residual_envelope: residual(),
        }
    }

    #[test]
    fn complete_declared_portfolio_is_not_claimed_complete_for_reality() {
        let manifest = manifest();
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(
            certificate.status,
            PortfolioStatus::PortfolioCompleteForDeclaredScope
        );
        assert_eq!(certificate.declared_hypothesis_count, 2);
        assert_eq!(certificate.effective_common_mode_family_count, 2);
        assert_eq!(certificate.no_counterexample_run_count, 1);
        assert_eq!(
            certificate.claim_ceiling,
            id("empirical-within-declared-scope-only")
        );
        assert!(certificate.integrity_check());
        assert_eq!(certificate.verify_against(&manifest), Ok(()));
    }

    #[test]
    fn shared_upstream_roots_merge_model_clones_into_one_family() {
        let mut manifest = manifest();
        manifest.hypotheses.push(hypothesis("model-a-clone", [21; 32]));
        manifest.decision.hypotheses.push(DecisionHypothesis {
            hypothesis_id: id("model-a-clone"),
            reference_manifest_digest: [13; 32],
            outcomes: vec![
                HypothesisActionOutcome {
                    action_id: id("action-a"),
                    support: OutcomeSupport::Supported {
                        utility: ExactRational::new(8, 1).unwrap(),
                        evidence_digest: [15; 32],
                    },
                },
                HypothesisActionOutcome {
                    action_id: id("action-b"),
                    support: OutcomeSupport::Supported {
                        utility: ExactRational::new(4, 1).unwrap(),
                        evidence_digest: [16; 32],
                    },
                },
            ],
        });
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.declared_hypothesis_count, 3);
        assert_eq!(certificate.effective_common_mode_family_count, 2);
        assert_eq!(
            certificate
                .common_mode_groups
                .iter()
                .find(|group| group.members.contains(&id("model-a")))
                .unwrap()
                .members
                .len(),
            2
        );
    }

    #[test]
    fn unknown_coverage_is_not_counted_as_covered() {
        let mut manifest = manifest();
        manifest.coverage_obligations[0].disposition =
            CoverageDisposition::Unknown {
                reason_id: id("night-rain-combination-not-tested"),
            };
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, PortfolioStatus::PortfolioIncomplete);
        assert_eq!(certificate.coverage[0].unknown, 1);
        assert_eq!(certificate.coverage[0].covered, 0);
    }

    #[test]
    fn uncovered_critical_combination_is_not_hidden_by_attribute_coverage() {
        let mut manifest = manifest();
        manifest.coverage_obligations[0].scope_id =
            id("rain-alone-covered-but-night-rain-occlusion-missing");
        manifest.coverage_obligations[0].disposition =
            CoverageDisposition::NotCovered {
                reason_id: id("critical-combination-missing"),
            };
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, PortfolioStatus::PortfolioIncomplete);
        assert_eq!(certificate.coverage[0].not_covered, 1);
    }

    #[test]
    fn out_of_scope_coverage_is_reported_separately() {
        let mut manifest = manifest();
        manifest.coverage_obligations[0].disposition =
            CoverageDisposition::OutOfScope {
                reason_id: id("rural-unpaved-roads-not-in-odd"),
            };
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, PortfolioStatus::OutOfScope);
        assert_eq!(certificate.coverage[0].out_of_scope, 1);
    }

    #[test]
    fn a_blinded_counterexample_falsifies_the_declared_portfolio() {
        let mut manifest = manifest();
        manifest.challenge_runs[0].result = ChallengeResult::CounterexampleFound {
            counterexample_id: id("counterexample-0042"),
            counterexample_digest: [31; 32],
            new_mechanism_id: id("pedestrian-occlusion-cue-feedback"),
        };
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, PortfolioStatus::PortfolioFalsified);
        assert_eq!(certificate.discovered_counterexamples.len(), 1);
        assert_eq!(
            certificate.discovered_counterexamples[0].new_mechanism_id,
            id("pedestrian-occlusion-cue-feedback")
        );
    }

    #[test]
    fn no_counterexample_within_budget_is_not_a_proof_of_absence() {
        let certificate = manifest().evaluate().unwrap();
        assert_eq!(certificate.no_counterexample_run_count, 1);
        assert_eq!(
            certificate.claim_ceiling,
            id("empirical-within-declared-scope-only")
        );
    }

    #[test]
    fn unbounded_residual_prevents_declared_scope_completion() {
        let mut manifest = manifest();
        manifest.residual_envelope = ResidualEnvelope::Unbounded {
            reason_id: id("unknown-mechanism-utility-not-bounded"),
        };
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, PortfolioStatus::ResidualRiskUnbounded);
        assert!(!certificate.residual_risk.bounded);
    }

    #[test]
    fn simulation_only_residual_cannot_support_empirical_portfolio_completion() {
        let mut manifest = manifest();
        if let ResidualEnvelope::Bounded { origin, .. } = &mut manifest.residual_envelope {
            *origin = ResidualEvidenceOrigin::SimulationOnly;
        }
        assert_eq!(
            manifest.evaluate().unwrap().status,
            PortfolioStatus::ResidualRiskUnbounded
        );
    }

    #[test]
    fn unknown_or_potential_residual_feasibility_excludes_action() {
        let mut manifest = manifest();
        if let ResidualEnvelope::Bounded { bounds, .. } = &mut manifest.residual_envelope {
            bounds[0].feasibility = ResidualFeasibility::Unknown {
                reason_id: id("unknown-residual-collision-risk"),
            };
        }
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, PortfolioStatus::ResidualRiskUnbounded);
        assert_eq!(
            certificate.residual_risk.excluded_actions,
            vec![id("action-a")]
        );
    }

    #[test]
    fn append_only_revision_requires_exact_parent_digest() {
        let parent = manifest();
        let mut child = parent.clone();
        child.revision = 2;
        child.parent_portfolio_digest = Some(parent.digest().unwrap());
        child.coverage_obligations.push(CoverageObligation {
            obligation_id: id("new-causal-mechanism-coverage"),
            dimension: CoverageDimension::CausalMechanism,
            scope_id: id("reciprocal-actor-ego-feedback"),
            disposition: CoverageDisposition::Covered {
                evidence_digest: [33; 32],
                protocol_digest: [34; 32],
                independent_unit_count: 12,
                challenge_run_ids: BTreeSet::from([id("challenge-main")]),
            },
        });
        assert_eq!(child.verify_successor_of(&parent), Ok(()));
        child.parent_portfolio_digest = Some([99; 32]);
        assert_eq!(
            child.verify_successor_of(&parent),
            Err(PortfolioMutationError::ParentDigestMismatch)
        );
    }

    #[test]
    fn missing_coverage_challenge_or_shared_lineage_rejects() {
        let mut no_challenge = manifest();
        if let CoverageDisposition::Covered { challenge_run_ids, .. } =
            &mut no_challenge.coverage_obligations[0].disposition
        {
            challenge_run_ids.clear();
        }
        assert_eq!(
            no_challenge.validate(),
            Err(PortfolioError::MissingCoverageChallenge)
        );

        let mut missing_lineage = manifest();
        missing_lineage.hypotheses[0].common_mode_root_digests.clear();
        assert_eq!(
            missing_lineage.validate(),
            Err(PortfolioError::EmptyCommonModeRoots)
        );
    }

    #[test]
    fn unknown_lineage_and_unblinded_challenges_do_not_pass() {
        let mut unblinded = manifest();
        unblinded.challenge_runs[0].blinded = false;
        assert_eq!(
            unblinded.validate(),
            Err(PortfolioError::ChallengeNotBlinded)
        );

        let mut bad_lineage = manifest();
        bad_lineage.hypotheses[0].hypothesis_revision_digest = [0; 32];
        assert_eq!(
            bad_lineage.validate(),
            Err(PortfolioError::ZeroHypothesisRevisionDigest)
        );
    }

    #[test]
    fn invalid_residual_interval_and_missing_action_bounds_reject() {
        let mut invalid_interval = manifest();
        if let ResidualEnvelope::Bounded { bounds, .. } = &mut invalid_interval.residual_envelope {
            bounds[0].utility_lower = ExactRational::new(10, 1).unwrap();
            bounds[0].utility_upper = ExactRational::new(9, 1).unwrap();
        }
        assert_eq!(
            invalid_interval.validate(),
            Err(PortfolioError::InvalidResidualInterval)
        );

        let mut missing_action = manifest();
        if let ResidualEnvelope::Bounded { bounds, .. } = &mut missing_action.residual_envelope {
            bounds.pop();
        }
        assert_eq!(
            missing_action.validate(),
            Err(PortfolioError::MissingResidualAction)
        );
    }

    #[test]
    fn formal_and_simulation_claim_scopes_require_matching_residual_origin() {
        let mut formal = manifest();
        formal.claim_scope = PortfolioClaimScope::FormalConformance;
        if let ResidualEnvelope::Bounded { origin, .. } = &mut formal.residual_envelope {
            *origin = ResidualEvidenceOrigin::FormalAnalyticBound;
        }
        assert_eq!(
            formal.evaluate().unwrap().status,
            PortfolioStatus::PortfolioCompleteForDeclaredScope
        );

        let mut simulation = manifest();
        simulation.claim_scope = PortfolioClaimScope::SimulationChallenge;
        if let ResidualEnvelope::Bounded { origin, .. } = &mut simulation.residual_envelope {
            *origin = ResidualEvidenceOrigin::SimulationOnly;
        }
        assert_eq!(
            simulation.evaluate().unwrap().status,
            PortfolioStatus::PortfolioCompleteForDeclaredScope
        );
        assert_eq!(
            simulation.evaluate().unwrap().claim_ceiling,
            id("simulation-challenge-only")
        );
    }

    #[test]
    fn declaration_order_does_not_change_portfolio_digest_or_certificate() {
        let first = manifest();
        let mut second = first.clone();
        second.hypotheses.reverse();
        second.coverage_obligations.reverse();
        second.challenge_runs.reverse();
        if let ResidualEnvelope::Bounded { bounds, .. } = &mut second.residual_envelope {
            bounds.reverse();
        }
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
        assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
    }

    #[test]
    fn source_root_overlap_forms_transitive_common_mode_groups() {
        let mut manifest = manifest();
        manifest.hypotheses = vec![
            HypothesisLineageRecord {
                hypothesis_id: id("h-a"),
                hypothesis_revision_digest: [1; 32],
                common_mode_root_digests: BTreeSet::from([[1; 32], [2; 32]]),
            },
            HypothesisLineageRecord {
                hypothesis_id: id("h-b"),
                hypothesis_revision_digest: [2; 32],
                common_mode_root_digests: BTreeSet::from([[2; 32], [3; 32]]),
            },
            HypothesisLineageRecord {
                hypothesis_id: id("h-c"),
                hypothesis_revision_digest: [3; 32],
                common_mode_root_digests: BTreeSet::from([[9; 32]]),
            },
        ];
        manifest.decision.hypotheses = vec![
            DecisionHypothesis {
                hypothesis_id: id("h-a"),
                reference_manifest_digest: [21; 32],
                outcomes: vec![
                    HypothesisActionOutcome {
                        action_id: id("action-a"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(8, 1).unwrap(),
                            evidence_digest: [1; 32],
                        },
                    },
                    HypothesisActionOutcome {
                        action_id: id("action-b"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(4, 1).unwrap(),
                            evidence_digest: [2; 32],
                        },
                    },
                ],
            },
            DecisionHypothesis {
                hypothesis_id: id("h-b"),
                reference_manifest_digest: [22; 32],
                outcomes: vec![
                    HypothesisActionOutcome {
                        action_id: id("action-a"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(7, 1).unwrap(),
                            evidence_digest: [3; 32],
                        },
                    },
                    HypothesisActionOutcome {
                        action_id: id("action-b"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(5, 1).unwrap(),
                            evidence_digest: [4; 32],
                        },
                    },
                ],
            },
            DecisionHypothesis {
                hypothesis_id: id("h-c"),
                reference_manifest_digest: [23; 32],
                outcomes: vec![
                    HypothesisActionOutcome {
                        action_id: id("action-a"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(6, 1).unwrap(),
                            evidence_digest: [5; 32],
                        },
                    },
                    HypothesisActionOutcome {
                        action_id: id("action-b"),
                        support: OutcomeSupport::Supported {
                            utility: ExactRational::new(5, 1).unwrap(),
                            evidence_digest: [6; 32],
                        },
                    },
                ],
            },
        ];
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.effective_common_mode_family_count, 2);
        let group = certificate
            .common_mode_groups
            .iter()
            .find(|group| group.members.contains(&id("h-a")))
            .unwrap();
        assert!(group.members.contains(&id("h-b")));
        assert!(!group.members.contains(&id("h-c")));
    }
}
