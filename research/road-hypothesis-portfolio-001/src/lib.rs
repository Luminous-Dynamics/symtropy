#![forbid(unsafe_code)]

//! Finite hypothesis-portfolio coverage, falsification and residual-risk oracle.
//!
//! This crate checks declarations and exact finite calculations. It cannot
//! establish completeness relative to reality, discover unknown mechanisms by
//! itself, or validate empirical residual bounds. Non-discovery is budgeted
//! evidence about a search, never proof that a counterexample does not exist.

use std::collections::{BTreeMap, BTreeSet};

const PORTFOLIO_SCHEMA: &str = "symtropy.road-hypothesis-portfolio.v0.1";
const HASH_DOMAIN: &str = "Symtropy ROAD-HYPOTHESIS-PORTFOLIO-001 v0.1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CoverageAxis {
    OddRange,
    BehaviouralCompetency,
    CausalMechanism,
    RuleSafety,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageState {
    Covered,
    NotCovered,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    ObservedFactual,
    ControlledIntervention,
    QualifiedNaturalExperiment,
    SimulationGenerated,
    ModelGenerated,
    FormalReference,
    HumanAdjudication,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageObligation {
    pub obligation_id: String,
    pub axis: CoverageAxis,
    /// Exact stratum or interaction-combination identity. A combination such
    /// as rain x night x occluded crossing is a separate obligation.
    pub stratum_digest: String,
    pub state: CoverageState,
    pub minimum_independent_units: u64,
    pub independent_unit_count: u64,
    pub evidence_class: Option<EvidenceClass>,
    pub evidence_manifest_digest: Option<String>,
    pub source_lineage_digest: Option<String>,
    pub challenge_protocol_digest: Option<String>,
    pub not_applicable_rationale: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExcludedMechanism {
    pub mechanism_id: String,
    pub rationale: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hypothesis {
    pub hypothesis_id: String,
    pub model_family_id: String,
    pub version_digest: String,
    pub mechanism_class_ids: Vec<String>,
    pub causal_graph_digest: String,
    pub stochastic_contract_digest: String,
    /// All known training, simulator, pretraining and source-data ancestors.
    pub source_lineage_digests: Vec<String>,
    /// Hypotheses in the same common-mode group count as one independent family.
    pub common_mode_group: Option<String>,
    pub applicability_scope_digest: String,
    pub excluded_mechanisms: Vec<ExcludedMechanism>,
    /// A changed hypothesis is a new identity and must explicitly supersede
    /// an older hypothesis; an existing record is never silently rewritten.
    pub supersedes_hypothesis_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeRequirement {
    pub hypothesis_id: String,
    pub strategy: ChallengeStrategy,
    pub protocol_digest: String,
    pub minimum_budget_units: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ChallengeStrategy {
    Random,
    CoverageGuided,
    Expert,
    ReinforcementLearning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChallengeOutcome {
    CounterexampleFound,
    NonDiscoveryWithinBudget,
    Incomplete,
    NotRun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeReceipt {
    pub challenge_id: String,
    pub hypothesis_id: String,
    pub strategy: ChallengeStrategy,
    pub protocol_digest: String,
    pub holdout_manifest_digest: String,
    pub blinded: bool,
    pub sealed_test: bool,
    pub budget_units: u64,
    pub holdout_parent_lineage_digests: Vec<String>,
    pub outcome: ChallengeOutcome,
    pub counterexample_digest: Option<String>,
    pub replay_receipt_digest: Option<String>,
    pub counterexample_replay_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortfolioManifest {
    pub schema: String,
    pub portfolio_id: String,
    pub revision_sequence: u64,
    pub supersedes_portfolio_digest: Option<String>,
    pub odd_scope_digest: String,
    pub utility_scale_id: String,
    pub utility_semantics_id: String,
    pub training_lineage_roots: Vec<String>,
    pub action_ids: Vec<String>,
    pub hypotheses: Vec<Hypothesis>,
    pub coverage_obligations: Vec<CoverageObligation>,
    pub challenge_requirements: Vec<ChallengeRequirement>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortfolioDisposition {
    PortfolioCompleteForDeclaredScope,
    PortfolioIncomplete,
    PortfolioFalsified,
    OutOfScope,
    ResidualRiskUnbounded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeasibilityDisposition {
    Guaranteed,
    PotentialViolation,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidualEvidenceClass {
    ObservedFactual,
    ControlledIntervention,
    QualifiedNaturalExperiment,
    SimulationGenerated,
    ModelGenerated,
    FormalReference,
    HumanAdjudication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rational {
    pub numerator: i64,
    pub denominator: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RationalError {
    ZeroDenominator,
    ArithmeticOverflow,
}

impl Rational {
    pub fn new(numerator: i64, denominator: u64) -> Result<Self, RationalError> {
        if denominator == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        let divisor = gcd_u64(numerator.unsigned_abs(), denominator);
        let reduced_numerator = i128::from(numerator) / i128::from(divisor);
        let numerator =
            i64::try_from(reduced_numerator).map_err(|_| RationalError::ArithmeticOverflow)?;
        Ok(Self {
            numerator,
            denominator: denominator / divisor,
        })
    }

    pub fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, RationalError> {
        if self.denominator == 0 || other.denominator == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        let common = gcd_u64(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = i128::from(self.numerator)
            .checked_mul(i128::from(left_factor))
            .ok_or(RationalError::ArithmeticOverflow)?;
        let right = i128::from(other.numerator)
            .checked_mul(i128::from(right_factor))
            .ok_or(RationalError::ArithmeticOverflow)?;
        let numerator = left
            .checked_sub(right)
            .ok_or(RationalError::ArithmeticOverflow)?;
        let denominator = u128::from(self.denominator)
            .checked_mul(u128::from(left_factor))
            .ok_or(RationalError::ArithmeticOverflow)?;
        rational_from_i128(numerator, denominator)
    }

    pub fn checked_add(self, other: Self) -> Result<Self, RationalError> {
        if self.denominator == 0 || other.denominator == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        let common = gcd_u64(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = i128::from(self.numerator)
            .checked_mul(i128::from(left_factor))
            .ok_or(RationalError::ArithmeticOverflow)?;
        let right = i128::from(other.numerator)
            .checked_mul(i128::from(right_factor))
            .ok_or(RationalError::ArithmeticOverflow)?;
        let numerator = left
            .checked_add(right)
            .ok_or(RationalError::ArithmeticOverflow)?;
        let denominator = u128::from(self.denominator)
            .checked_mul(u128::from(left_factor))
            .ok_or(RationalError::ArithmeticOverflow)?;
        rational_from_i128(numerator, denominator)
    }

    pub fn cmp_exact(self, other: Self) -> Result<std::cmp::Ordering, RationalError> {
        if self.denominator == 0 || other.denominator == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        let left = i128::from(self.numerator)
            .checked_mul(i128::from(other.denominator))
            .ok_or(RationalError::ArithmeticOverflow)?;
        let right = i128::from(other.numerator)
            .checked_mul(i128::from(self.denominator))
            .ok_or(RationalError::ArithmeticOverflow)?;
        Ok(left.cmp(&right))
    }

    pub fn max(self, other: Self) -> Result<Self, RationalError> {
        Ok(match self.cmp_exact(other)? {
            std::cmp::Ordering::Less => other,
            _ => self,
        })
    }

    pub fn min(self, other: Self) -> Result<Self, RationalError> {
        Ok(match self.cmp_exact(other)? {
            std::cmp::Ordering::Greater => other,
            _ => self,
        })
    }

    pub fn is_nonnegative(self) -> bool {
        self.numerator >= 0
    }
}

fn rational_from_i128(numerator: i128, denominator: u128) -> Result<Rational, RationalError> {
    if denominator == 0 {
        return Err(RationalError::ZeroDenominator);
    }
    let divisor = gcd_u128(numerator.unsigned_abs(), denominator);
    let reduced_num_abs = numerator.unsigned_abs() / divisor;
    let reduced_den = denominator / divisor;
    let signed_num = if numerator < 0 {
        if reduced_num_abs == (i64::MAX as u128) + 1 {
            i64::MIN
        } else {
            let positive =
                i64::try_from(reduced_num_abs).map_err(|_| RationalError::ArithmeticOverflow)?;
            -positive
        }
    } else {
        i64::try_from(reduced_num_abs).map_err(|_| RationalError::ArithmeticOverflow)?
    };
    let denominator = u64::try_from(reduced_den).map_err(|_| RationalError::ArithmeticOverflow)?;
    Ok(Rational {
        numerator: signed_num,
        denominator,
    })
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionResidualBound {
    pub action_id: String,
    pub utility_lower: Rational,
    pub utility_upper: Rational,
    pub evidence_digest: String,
    pub calibration_protocol_digest: String,
    pub source_lineage_digests: Vec<String>,
    pub minimum_independent_units: u64,
    pub independent_unit_count: u64,
    pub feasibility: FeasibilityDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualEnvelope {
    pub utility_scale_id: String,
    pub utility_semantics_id: String,
    /// Residual bounds are valid only for the exact ODD declared by the portfolio.
    pub scope_digest: String,
    /// The counterfactual quantity and independent experimental unit are explicit.
    pub estimand_digest: String,
    pub independent_unit_definition_digest: String,
    pub evidence_class: ResidualEvidenceClass,
    /// Controlled interventions need an assignment protocol; natural experiments
    /// need a separately reviewed identification-assumptions package.
    pub assignment_protocol_digest: Option<String>,
    pub identification_assumptions_digest: Option<String>,
    pub evidence_digest: String,
    pub calibration_protocol_digest: String,
    /// V0.1 implements the explicitly conservative rectangular relaxation.
    /// A non-rectangular claim requires a separate joint residual model.
    pub rectangular_relaxation: bool,
    pub action_bounds: Vec<ActionResidualBound>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidualDisposition {
    BoundedRectangularRelaxation,
    ResidualRiskUnbounded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRegretBound {
    pub action_id: String,
    pub worst_case_regret_upper: Rational,
    pub feasibility: FeasibilityDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AxisCoverageSummary {
    pub axis: CoverageAxis,
    pub declared_obligations: usize,
    pub covered_obligations: usize,
    pub not_applicable_obligations: usize,
    pub unresolved_obligations: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategySearchSummary {
    pub strategy: ChallengeStrategy,
    pub attempts: usize,
    pub total_budget_units: u64,
    pub non_discovery_budget_units: u64,
    pub counterexamples_found: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeSearchPoint {
    pub challenge_id: String,
    pub hypothesis_id: String,
    pub strategy: ChallengeStrategy,
    pub budget_units: u64,
    pub outcome: ChallengeOutcome,
    pub holdout_manifest_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeSummary {
    pub required_challenges: usize,
    pub qualified_challenges: usize,
    pub unresolved_challenges: usize,
    pub strategies: Vec<StrategySearchSummary>,
    pub search_points: Vec<ChallengeSearchPoint>,
    /// Always false: failure to find a counterexample is budgeted non-discovery,
    /// not proof of the absence of an omitted mechanism.
    pub non_discovery_proves_absence: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualAssessment {
    pub disposition: ResidualDisposition,
    pub action_regret_bounds: Vec<ActionRegretBound>,
    pub robust_feasible_actions: Vec<String>,
    pub minimax_regret_actions: Vec<String>,
    pub rectangular_bound_is_attainable_joint_world: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortfolioEvaluation {
    pub portfolio_digest: String,
    pub evaluation_digest: String,
    pub disposition: PortfolioDisposition,
    pub odd_scope_match: bool,
    pub model_family_count: usize,
    pub effective_independent_hypothesis_family_count: usize,
    pub axes: Vec<AxisCoverageSummary>,
    pub challenge: ChallengeSummary,
    pub residual: ResidualAssessment,
    pub decision_recommendation_authorized: bool,
    /// This reference never certifies completeness relative to reality.
    pub completeness_relative_to_reality_proven: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortfolioError {
    WrongSchema,
    EmptyIdentity(&'static str),
    EmptyPortfolio,
    DuplicateAction,
    DuplicateHypothesis,
    DuplicateCoverageObligation,
    DuplicateChallengeRequirement,
    DuplicateChallengeReceipt,
    UnrequiredChallengeReceipt,
    InvalidChallengeBudget,
    ChallengeHoldoutMismatch,
    ResidualEvidenceLineageLeak,
    InsufficientResidualUnits,
    ChallengeBudgetMismatch,
    MissingCoverageAxis,
    InvalidCoverageEvidence,
    InvalidNotApplicableRationale,
    InvalidHypothesisLineage,
    UnknownChallengeHypothesis,
    MissingChallengeParentLineage,
    ChallengeNotBlinded,
    ChallengeLineageLeak,
    CounterexampleMissingReplay,
    InvalidResidualBounds,
    ResidualActionCoverageMismatch,
    UtilityScaleMismatch,
    InvalidRevisionParent,
    NonAppendOnlyRevision,
    ArithmeticOverflow,
}

impl From<RationalError> for PortfolioError {
    fn from(value: RationalError) -> Self {
        match value {
            RationalError::ZeroDenominator => Self::InvalidResidualBounds,
            RationalError::ArithmeticOverflow => Self::ArithmeticOverflow,
        }
    }
}

impl PortfolioManifest {
    pub fn portfolio_digest(&self) -> Result<String, PortfolioError> {
        self.validate()?;
        let mut hasher = blake3::Hasher::new_derive_key(HASH_DOMAIN);
        hash_str(&mut hasher, &self.schema);
        hash_str(&mut hasher, &self.portfolio_id);
        hash_u64(&mut hasher, self.revision_sequence);
        hash_opt_str(&mut hasher, self.supersedes_portfolio_digest.as_deref());
        hash_str(&mut hasher, &self.odd_scope_digest);
        hash_str(&mut hasher, &self.utility_scale_id);
        hash_str(&mut hasher, &self.utility_semantics_id);

        let mut training = self.training_lineage_roots.clone();
        training.sort();
        hash_strings(&mut hasher, &training);

        let mut actions = self.action_ids.clone();
        actions.sort();
        hash_strings(&mut hasher, &actions);

        let mut hypotheses: Vec<_> = self.hypotheses.iter().collect();
        hypotheses.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
        hash_len(&mut hasher, hypotheses.len());
        for hypothesis in hypotheses {
            hash_hypothesis(&mut hasher, hypothesis);
        }

        let mut obligations: Vec<_> = self.coverage_obligations.iter().collect();
        obligations.sort_by(|a, b| a.obligation_id.cmp(&b.obligation_id));
        hash_len(&mut hasher, obligations.len());
        for obligation in obligations {
            hash_coverage(&mut hasher, obligation);
        }

        let mut requirements: Vec<_> = self.challenge_requirements.iter().collect();
        requirements.sort_by(|a, b| {
            (&a.hypothesis_id, a.strategy, &a.protocol_digest).cmp(&(
                &b.hypothesis_id,
                b.strategy,
                &b.protocol_digest,
            ))
        });
        hash_len(&mut hasher, requirements.len());
        for requirement in requirements {
            hash_str(&mut hasher, &requirement.hypothesis_id);
            hash_u8(&mut hasher, strategy_tag(requirement.strategy));
            hash_str(&mut hasher, &requirement.protocol_digest);
            hash_u64(&mut hasher, requirement.minimum_budget_units);
        }
        Ok(hex(hasher.finalize().as_bytes()))
    }

    pub fn validate(&self) -> Result<(), PortfolioError> {
        if self.schema != PORTFOLIO_SCHEMA {
            return Err(PortfolioError::WrongSchema);
        }
        require(&self.portfolio_id, "portfolio_id")?;
        require(&self.odd_scope_digest, "odd_scope_digest")?;
        require(&self.utility_scale_id, "utility_scale_id")?;
        require(&self.utility_semantics_id, "utility_semantics_id")?;
        let mut training_roots = BTreeSet::new();
        for root in &self.training_lineage_roots {
            require(root, "training_lineage_root")?;
            if !training_roots.insert(root.as_str()) {
                return Err(PortfolioError::InvalidHypothesisLineage);
            }
        }
        if self.hypotheses.is_empty()
            || self.action_ids.is_empty()
            || self.coverage_obligations.is_empty()
            || self.challenge_requirements.is_empty()
        {
            return Err(PortfolioError::EmptyPortfolio);
        }

        let mut action_ids = BTreeSet::new();
        for action in &self.action_ids {
            require(action, "action_id")?;
            if !action_ids.insert(action.as_str()) {
                return Err(PortfolioError::DuplicateAction);
            }
        }

        let mut hypothesis_ids = BTreeSet::new();
        for hypothesis in &self.hypotheses {
            require(&hypothesis.hypothesis_id, "hypothesis_id")?;
            require(&hypothesis.model_family_id, "model_family_id")?;
            require(&hypothesis.version_digest, "version_digest")?;
            require(&hypothesis.causal_graph_digest, "causal_graph_digest")?;
            if hypothesis.mechanism_class_ids.is_empty() {
                return Err(PortfolioError::InvalidHypothesisLineage);
            }
            for class_id in &hypothesis.mechanism_class_ids {
                require(class_id, "mechanism_class_id")?;
            }
            require(
                &hypothesis.stochastic_contract_digest,
                "stochastic_contract_digest",
            )?;
            require(
                &hypothesis.applicability_scope_digest,
                "applicability_scope_digest",
            )?;
            if hypothesis.source_lineage_digests.is_empty() {
                return Err(PortfolioError::InvalidHypothesisLineage);
            }
            for lineage in &hypothesis.source_lineage_digests {
                require(lineage, "source_lineage_digest")?;
            }
            if let Some(group) = &hypothesis.common_mode_group {
                require(group, "common_mode_group")?;
            }
            for excluded in &hypothesis.excluded_mechanisms {
                require(&excluded.mechanism_id, "excluded_mechanism_id")?;
                require(&excluded.rationale, "excluded_mechanism_rationale")?;
            }
            if let Some(previous) = &hypothesis.supersedes_hypothesis_id {
                require(previous, "supersedes_hypothesis_id")?;
            }
            if !hypothesis_ids.insert(hypothesis.hypothesis_id.as_str()) {
                return Err(PortfolioError::DuplicateHypothesis);
            }
        }

        let mut obligation_ids = BTreeSet::new();
        let mut axes = BTreeSet::new();
        for obligation in &self.coverage_obligations {
            require(&obligation.obligation_id, "obligation_id")?;
            require(&obligation.stratum_digest, "stratum_digest")?;
            axes.insert(obligation.axis);
            if !obligation_ids.insert(obligation.obligation_id.as_str()) {
                return Err(PortfolioError::DuplicateCoverageObligation);
            }
            validate_coverage_obligation(obligation)?;
        }
        for axis in [
            CoverageAxis::OddRange,
            CoverageAxis::BehaviouralCompetency,
            CoverageAxis::CausalMechanism,
            CoverageAxis::RuleSafety,
        ] {
            if !axes.contains(&axis) {
                return Err(PortfolioError::MissingCoverageAxis);
            }
        }

        let mut requirement_keys = BTreeSet::new();
        let mut required_budgets = BTreeSet::new();
        for requirement in &self.challenge_requirements {
            require(&requirement.hypothesis_id, "challenge.hypothesis_id")?;
            require(&requirement.protocol_digest, "challenge.protocol_digest")?;
            if !hypothesis_ids.contains(requirement.hypothesis_id.as_str()) {
                return Err(PortfolioError::UnknownChallengeHypothesis);
            }
            if requirement.minimum_budget_units == 0 {
                return Err(PortfolioError::InvalidChallengeBudget);
            }
            required_budgets.insert(requirement.minimum_budget_units);
            let key = (
                requirement.hypothesis_id.as_str(),
                requirement.strategy,
                requirement.protocol_digest.as_str(),
            );
            if !requirement_keys.insert(key) {
                return Err(PortfolioError::DuplicateChallengeRequirement);
            }
        }
        if required_budgets.len() != 1 {
            return Err(PortfolioError::ChallengeBudgetMismatch);
        }
        for hypothesis in &self.hypotheses {
            for strategy in [
                ChallengeStrategy::Random,
                ChallengeStrategy::CoverageGuided,
                ChallengeStrategy::Expert,
                ChallengeStrategy::ReinforcementLearning,
            ] {
                if !self.challenge_requirements.iter().any(|requirement| {
                    requirement.hypothesis_id == hypothesis.hypothesis_id
                        && requirement.strategy == strategy
                }) {
                    return Err(PortfolioError::UnknownChallengeHypothesis);
                }
            }
        }
        if let Some(parent) = &self.supersedes_portfolio_digest {
            require(parent, "supersedes_portfolio_digest")?;
        }
        Ok(())
    }

    /// Require a new portfolio digest and a strictly append-only hypothesis /
    /// obligation / challenge contract. An old challenged hypothesis cannot be
    /// edited in place; the new manifest must preserve it and add a successor.
    pub fn validate_append_only_revision(
        previous: &PortfolioManifest,
        current: &PortfolioManifest,
    ) -> Result<(), PortfolioError> {
        previous.validate()?;
        current.validate()?;
        let previous_digest = previous.portfolio_digest()?;
        let expected_sequence = previous
            .revision_sequence
            .checked_add(1)
            .ok_or(PortfolioError::InvalidRevisionParent)?;
        if current.supersedes_portfolio_digest.as_deref() != Some(previous_digest.as_str())
            || current.revision_sequence != expected_sequence
            || current.portfolio_id != previous.portfolio_id
            || current.utility_scale_id != previous.utility_scale_id
            || current.utility_semantics_id != previous.utility_semantics_id
        {
            return Err(PortfolioError::InvalidRevisionParent);
        }

        for prior in &previous.hypotheses {
            if !current
                .hypotheses
                .iter()
                .any(|candidate| candidate == prior)
            {
                return Err(PortfolioError::NonAppendOnlyRevision);
            }
        }
        for prior in &previous.coverage_obligations {
            if !current
                .coverage_obligations
                .iter()
                .any(|candidate| candidate == prior)
            {
                return Err(PortfolioError::NonAppendOnlyRevision);
            }
        }
        for prior in &previous.challenge_requirements {
            if !current
                .challenge_requirements
                .iter()
                .any(|candidate| candidate == prior)
            {
                return Err(PortfolioError::NonAppendOnlyRevision);
            }
        }
        for prior_action in &previous.action_ids {
            if !current.action_ids.contains(prior_action) {
                return Err(PortfolioError::NonAppendOnlyRevision);
            }
        }
        for prior_lineage in &previous.training_lineage_roots {
            if !current.training_lineage_roots.contains(prior_lineage) {
                return Err(PortfolioError::NonAppendOnlyRevision);
            }
        }

        let prior_ids: BTreeSet<_> = previous
            .hypotheses
            .iter()
            .map(|hypothesis| hypothesis.hypothesis_id.as_str())
            .collect();
        for added in &current.hypotheses {
            if !prior_ids.contains(added.hypothesis_id.as_str()) {
                if added
                    .supersedes_hypothesis_id
                    .as_ref()
                    .is_some_and(|id| !prior_ids.contains(id.as_str()))
                {
                    return Err(PortfolioError::NonAppendOnlyRevision);
                }
            }
        }
        Ok(())
    }
}

fn validate_coverage_obligation(obligation: &CoverageObligation) -> Result<(), PortfolioError> {
    match obligation.state {
        CoverageState::Covered => {
            let evidence = obligation.evidence_manifest_digest.as_deref().unwrap_or("");
            let lineage = obligation.source_lineage_digest.as_deref().unwrap_or("");
            let protocol = obligation
                .challenge_protocol_digest
                .as_deref()
                .unwrap_or("");
            require(evidence, "coverage.evidence_manifest_digest")?;
            require(lineage, "coverage.source_lineage_digest")?;
            require(protocol, "coverage.challenge_protocol_digest")?;
            if obligation.evidence_class.is_none() {
                return Err(PortfolioError::InvalidCoverageEvidence);
            }
            if obligation.minimum_independent_units == 0
                || obligation.independent_unit_count < obligation.minimum_independent_units
            {
                return Err(PortfolioError::InvalidCoverageEvidence);
            }
            if obligation.not_applicable_rationale.is_some() {
                return Err(PortfolioError::InvalidCoverageEvidence);
            }
        }
        CoverageState::NotApplicable => {
            let rationale = obligation.not_applicable_rationale.as_deref().unwrap_or("");
            require(rationale, "coverage.not_applicable_rationale")?;
        }
        CoverageState::NotCovered | CoverageState::Unknown => {}
    }
    Ok(())
}

impl Rational {
    pub fn checked_regret_upper(
        rival_upper: Rational,
        selected_lower: Rational,
    ) -> Result<Rational, RationalError> {
        let delta = rival_upper.checked_sub(selected_lower)?;
        Ok(if delta.is_nonnegative() {
            delta
        } else {
            Rational::zero()
        })
    }
}

pub fn evaluate_portfolio(
    manifest: &PortfolioManifest,
    evaluation_odd_scope_digest: &str,
    challenge_receipts: &[ChallengeReceipt],
    residual_envelope: Option<&ResidualEnvelope>,
) -> Result<PortfolioEvaluation, PortfolioError> {
    manifest.validate()?;
    require(evaluation_odd_scope_digest, "evaluation_odd_scope_digest")?;
    let digest = manifest.portfolio_digest()?;
    let odd_scope_match = evaluation_odd_scope_digest == manifest.odd_scope_digest;

    let model_family_count = manifest
        .hypotheses
        .iter()
        .map(|hypothesis| hypothesis.model_family_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let independent_families = independent_hypothesis_families(&manifest.hypotheses);
    let axes = coverage_summaries(&manifest.coverage_obligations);

    let (challenge, falsified) = assess_challenges(manifest, challenge_receipts)?;
    let coverage_complete = axes.iter().all(|summary| {
        summary.unresolved_obligations == 0
            && summary.declared_obligations
                == summary.covered_obligations + summary.not_applicable_obligations
    });
    let challenges_complete = challenge.unresolved_challenges == 0;

    let residual = assess_residual(manifest, residual_envelope, challenge_receipts)?;
    let disposition = if !odd_scope_match {
        PortfolioDisposition::OutOfScope
    } else if falsified {
        PortfolioDisposition::PortfolioFalsified
    } else if !coverage_complete || !challenges_complete {
        PortfolioDisposition::PortfolioIncomplete
    } else if residual.disposition == ResidualDisposition::ResidualRiskUnbounded {
        PortfolioDisposition::ResidualRiskUnbounded
    } else {
        PortfolioDisposition::PortfolioCompleteForDeclaredScope
    };

    let decision_recommendation_authorized = disposition
        == PortfolioDisposition::PortfolioCompleteForDeclaredScope
        && residual.disposition == ResidualDisposition::BoundedRectangularRelaxation
        && !residual.robust_feasible_actions.is_empty();
    let evaluation_digest = evaluation_digest(
        &digest,
        evaluation_odd_scope_digest,
        challenge_receipts,
        residual_envelope,
        disposition,
        decision_recommendation_authorized,
    );

    Ok(PortfolioEvaluation {
        portfolio_digest: digest,
        evaluation_digest,
        disposition,
        odd_scope_match,
        model_family_count,
        effective_independent_hypothesis_family_count: independent_families.len(),
        axes,
        challenge,
        residual,
        decision_recommendation_authorized,
        completeness_relative_to_reality_proven: false,
    })
}

fn evaluation_digest(
    portfolio_digest: &str,
    evaluation_odd_scope_digest: &str,
    receipts: &[ChallengeReceipt],
    residual: Option<&ResidualEnvelope>,
    disposition: PortfolioDisposition,
    recommendation_authorized: bool,
) -> String {
    let mut h = blake3::Hasher::new_derive_key(
        "Symtropy ROAD-HYPOTHESIS-PORTFOLIO evaluation receipt v0.1",
    );
    hash_str(&mut h, portfolio_digest);
    hash_str(&mut h, evaluation_odd_scope_digest);
    hash_u8(
        &mut h,
        match disposition {
            PortfolioDisposition::PortfolioCompleteForDeclaredScope => 0,
            PortfolioDisposition::PortfolioIncomplete => 1,
            PortfolioDisposition::PortfolioFalsified => 2,
            PortfolioDisposition::OutOfScope => 3,
            PortfolioDisposition::ResidualRiskUnbounded => 4,
        },
    );
    hash_u8(&mut h, u8::from(recommendation_authorized));

    let mut ordered_receipts: Vec<_> = receipts.iter().collect();
    ordered_receipts.sort_by(|a, b| a.challenge_id.cmp(&b.challenge_id));
    hash_len(&mut h, ordered_receipts.len());
    for receipt in ordered_receipts {
        hash_str(&mut h, &receipt.challenge_id);
        hash_str(&mut h, &receipt.hypothesis_id);
        hash_u8(&mut h, strategy_tag(receipt.strategy));
        hash_str(&mut h, &receipt.protocol_digest);
        hash_str(&mut h, &receipt.holdout_manifest_digest);
        hash_u8(&mut h, u8::from(receipt.blinded));
        hash_u8(&mut h, u8::from(receipt.sealed_test));
        hash_u64(&mut h, receipt.budget_units);
        let mut parents = receipt.holdout_parent_lineage_digests.clone();
        parents.sort();
        hash_strings(&mut h, &parents);
        hash_u8(
            &mut h,
            match receipt.outcome {
                ChallengeOutcome::CounterexampleFound => 0,
                ChallengeOutcome::NonDiscoveryWithinBudget => 1,
                ChallengeOutcome::Incomplete => 2,
                ChallengeOutcome::NotRun => 3,
            },
        );
        hash_opt_str(&mut h, receipt.counterexample_digest.as_deref());
        hash_opt_str(&mut h, receipt.replay_receipt_digest.as_deref());
        hash_u8(&mut h, u8::from(receipt.counterexample_replay_verified));
    }

    match residual {
        Some(envelope) => {
            hash_u8(&mut h, 1);
            hash_str(&mut h, &envelope.utility_scale_id);
            hash_str(&mut h, &envelope.utility_semantics_id);
            hash_str(&mut h, &envelope.scope_digest);
            hash_str(&mut h, &envelope.estimand_digest);
            hash_str(&mut h, &envelope.independent_unit_definition_digest);
            hash_opt_str(&mut h, envelope.assignment_protocol_digest.as_deref());
            hash_opt_str(&mut h, envelope.identification_assumptions_digest.as_deref());
            hash_u8(
                &mut h,
                match envelope.evidence_class {
                    ResidualEvidenceClass::ObservedFactual => 0,
                    ResidualEvidenceClass::ControlledIntervention => 1,
                    ResidualEvidenceClass::QualifiedNaturalExperiment => 2,
                    ResidualEvidenceClass::SimulationGenerated => 3,
                    ResidualEvidenceClass::ModelGenerated => 4,
                    ResidualEvidenceClass::FormalReference => 5,
                    ResidualEvidenceClass::HumanAdjudication => 6,
                },
            );
            hash_str(&mut h, &envelope.evidence_digest);
            hash_str(&mut h, &envelope.calibration_protocol_digest);
            hash_u8(&mut h, u8::from(envelope.rectangular_relaxation));
            let mut bounds: Vec<_> = envelope.action_bounds.iter().collect();
            bounds.sort_by(|a, b| a.action_id.cmp(&b.action_id));
            hash_len(&mut h, bounds.len());
            for bound in bounds {
                hash_str(&mut h, &bound.action_id);
                hash_i64(&mut h, bound.utility_lower.numerator);
                hash_u64(&mut h, bound.utility_lower.denominator);
                hash_i64(&mut h, bound.utility_upper.numerator);
                hash_u64(&mut h, bound.utility_upper.denominator);
                hash_str(&mut h, &bound.evidence_digest);
                hash_str(&mut h, &bound.calibration_protocol_digest);
                let mut lineage = bound.source_lineage_digests.clone();
                lineage.sort();
                hash_strings(&mut h, &lineage);
                hash_u64(&mut h, bound.minimum_independent_units);
                hash_u64(&mut h, bound.independent_unit_count);
                hash_u8(
                    &mut h,
                    match bound.feasibility {
                        FeasibilityDisposition::Guaranteed => 0,
                        FeasibilityDisposition::PotentialViolation => 1,
                        FeasibilityDisposition::Unknown => 2,
                    },
                );
            }
        }
        None => hash_u8(&mut h, 0),
    }
    hex(h.finalize().as_bytes())
}

fn independent_hypothesis_families(hypotheses: &[Hypothesis]) -> BTreeSet<usize> {
    let mut parents: Vec<usize> = (0..hypotheses.len()).collect();
    for left in 0..hypotheses.len() {
        for right in (left + 1)..hypotheses.len() {
            let a = &hypotheses[left];
            let b = &hypotheses[right];
            let same_common_mode_group =
                a.common_mode_group.is_some() && a.common_mode_group == b.common_mode_group;
            let shared_ancestor = a
                .source_lineage_digests
                .iter()
                .any(|source| b.source_lineage_digests.contains(source));
            if same_common_mode_group || shared_ancestor {
                union(&mut parents, left, right);
            }
        }
    }
    (0..parents.len())
        .map(|index| find_root(&mut parents, index))
        .collect()
}

fn find_root(parents: &mut [usize], index: usize) -> usize {
    let mut root = index;
    while parents[root] != root {
        root = parents[root];
    }
    let mut current = index;
    while parents[current] != current {
        let next = parents[current];
        parents[current] = root;
        current = next;
    }
    root
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left_root = find_root(parents, left);
    let right_root = find_root(parents, right);
    if left_root != right_root {
        let (root, child) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        parents[child] = root;
    }
}

fn coverage_summaries(obligations: &[CoverageObligation]) -> Vec<AxisCoverageSummary> {
    [
        CoverageAxis::OddRange,
        CoverageAxis::BehaviouralCompetency,
        CoverageAxis::CausalMechanism,
        CoverageAxis::RuleSafety,
    ]
    .into_iter()
    .map(|axis| {
        let matching: Vec<_> = obligations.iter().filter(|o| o.axis == axis).collect();
        AxisCoverageSummary {
            axis,
            declared_obligations: matching.len(),
            covered_obligations: matching
                .iter()
                .filter(|o| o.state == CoverageState::Covered)
                .count(),
            not_applicable_obligations: matching
                .iter()
                .filter(|o| o.state == CoverageState::NotApplicable)
                .count(),
            unresolved_obligations: matching
                .iter()
                .filter(|o| {
                    !matches!(
                        o.state,
                        CoverageState::Covered | CoverageState::NotApplicable
                    )
                })
                .count(),
        }
    })
    .collect()
}

fn assess_challenges(
    manifest: &PortfolioManifest,
    receipts: &[ChallengeReceipt],
) -> Result<(ChallengeSummary, bool), PortfolioError> {
    let training: BTreeSet<_> =
        manifest
            .training_lineage_roots
            .iter()
            .map(String::as_str)
            .chain(manifest.hypotheses.iter().flat_map(|hypothesis| {
                hypothesis.source_lineage_digests.iter().map(String::as_str)
            }))
            .collect();

    let mut receipt_map = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let required_keys: BTreeSet<_> = manifest
        .challenge_requirements
        .iter()
        .map(|requirement| {
            (
                requirement.hypothesis_id.as_str(),
                requirement.strategy,
                requirement.protocol_digest.as_str(),
            )
        })
        .collect();
    for receipt in receipts {
        require(&receipt.challenge_id, "challenge_id")?;
        require(&receipt.hypothesis_id, "challenge.hypothesis_id")?;
        require(&receipt.protocol_digest, "challenge.protocol_digest")?;
        require(
            &receipt.holdout_manifest_digest,
            "challenge.holdout_manifest_digest",
        )?;
        if !ids.insert(receipt.challenge_id.as_str()) {
            return Err(PortfolioError::DuplicateChallengeReceipt);
        }
        if receipt.holdout_parent_lineage_digests.is_empty() {
            return Err(PortfolioError::MissingChallengeParentLineage);
        }
        for parent in &receipt.holdout_parent_lineage_digests {
            require(parent, "challenge.holdout_parent_lineage")?;
            if training.contains(parent.as_str()) {
                return Err(PortfolioError::ChallengeLineageLeak);
            }
        }
        if !receipt.blinded || !receipt.sealed_test {
            return Err(PortfolioError::ChallengeNotBlinded);
        }
        if receipt.outcome == ChallengeOutcome::CounterexampleFound {
            require(
                receipt.counterexample_digest.as_deref().unwrap_or(""),
                "challenge.counterexample_digest",
            )?;
            require(
                receipt.replay_receipt_digest.as_deref().unwrap_or(""),
                "challenge.replay_receipt_digest",
            )?;
            if !receipt.counterexample_replay_verified {
                return Err(PortfolioError::CounterexampleMissingReplay);
            }
        }
        if receipt.outcome != ChallengeOutcome::CounterexampleFound
            && (receipt.counterexample_digest.is_some()
                || receipt.replay_receipt_digest.is_some()
                || receipt.counterexample_replay_verified)
        {
            return Err(PortfolioError::CounterexampleMissingReplay);
        }
        let key = (
            receipt.hypothesis_id.as_str(),
            receipt.strategy,
            receipt.protocol_digest.as_str(),
        );
        if !required_keys.contains(&key) {
            return Err(PortfolioError::UnrequiredChallengeReceipt);
        }
        if receipt_map.insert(key, receipt).is_some() {
            return Err(PortfolioError::DuplicateChallengeReceipt);
        }
    }

    let holdout_ids: BTreeSet<_> = receipts
        .iter()
        .map(|receipt| receipt.holdout_manifest_digest.as_str())
        .collect();
    let search_budgets: BTreeSet<_> = receipts
        .iter()
        .map(|receipt| receipt.budget_units)
        .collect();
    let holdout_parent_sets: BTreeSet<Vec<String>> = receipts
        .iter()
        .map(|receipt| {
            let mut parents = receipt.holdout_parent_lineage_digests.clone();
            parents.sort();
            parents.dedup();
            parents
        })
        .collect();
    if holdout_ids.len() > 1 || holdout_parent_sets.len() > 1 {
        return Err(PortfolioError::ChallengeHoldoutMismatch);
    }
    if search_budgets.len() > 1 {
        return Err(PortfolioError::ChallengeBudgetMismatch);
    }

    let mut unresolved = 0usize;
    let mut qualified = 0usize;
    let mut falsified = false;
    let mut by_strategy: BTreeMap<ChallengeStrategy, StrategySearchSummary> = BTreeMap::new();
    let mut search_points = Vec::with_capacity(receipts.len());

    for requirement in &manifest.challenge_requirements {
        let entry = by_strategy
            .entry(requirement.strategy)
            .or_insert(StrategySearchSummary {
                strategy: requirement.strategy,
                attempts: 0,
                total_budget_units: 0,
                non_discovery_budget_units: 0,
                counterexamples_found: 0,
            });
        let key = (
            requirement.hypothesis_id.as_str(),
            requirement.strategy,
            requirement.protocol_digest.as_str(),
        );
        if let Some(receipt) = receipt_map.get(&key) {
            search_points.push(ChallengeSearchPoint {
                challenge_id: receipt.challenge_id.clone(),
                hypothesis_id: receipt.hypothesis_id.clone(),
                strategy: receipt.strategy,
                budget_units: receipt.budget_units,
                outcome: receipt.outcome,
                holdout_manifest_digest: receipt.holdout_manifest_digest.clone(),
            });
            entry.attempts += 1;
            entry.total_budget_units = entry
                .total_budget_units
                .checked_add(receipt.budget_units)
                .ok_or(PortfolioError::ArithmeticOverflow)?;
            match receipt.outcome {
                ChallengeOutcome::CounterexampleFound => {
                    entry.counterexamples_found += 1;
                    falsified = true;
                    qualified += 1;
                }
                ChallengeOutcome::NonDiscoveryWithinBudget => {
                    if receipt.budget_units >= requirement.minimum_budget_units {
                        entry.non_discovery_budget_units = entry
                            .non_discovery_budget_units
                            .checked_add(receipt.budget_units)
                            .ok_or(PortfolioError::ArithmeticOverflow)?;
                        qualified += 1;
                    } else {
                        unresolved += 1;
                    }
                }
                ChallengeOutcome::Incomplete | ChallengeOutcome::NotRun => unresolved += 1,
            }
        } else {
            unresolved += 1;
        }
    }

    Ok((
        ChallengeSummary {
            required_challenges: manifest.challenge_requirements.len(),
            qualified_challenges: qualified,
            unresolved_challenges: unresolved,
            strategies: by_strategy.into_values().collect(),
            search_points,
            non_discovery_proves_absence: false,
        },
        falsified,
    ))
}

fn assess_residual(
    manifest: &PortfolioManifest,
    envelope: Option<&ResidualEnvelope>,
    challenge_receipts: &[ChallengeReceipt],
) -> Result<ResidualAssessment, PortfolioError> {
    let unbounded = || ResidualAssessment {
        disposition: ResidualDisposition::ResidualRiskUnbounded,
        action_regret_bounds: Vec::new(),
        robust_feasible_actions: Vec::new(),
        minimax_regret_actions: Vec::new(),
        rectangular_bound_is_attainable_joint_world: false,
    };
    let Some(envelope) = envelope else {
        return Ok(unbounded());
    };

    if envelope.utility_scale_id != manifest.utility_scale_id
        || envelope.utility_semantics_id != manifest.utility_semantics_id
    {
        return Err(PortfolioError::UtilityScaleMismatch);
    }
    let evidence_identification_qualified = match envelope.evidence_class {
        ResidualEvidenceClass::ControlledIntervention => {
            envelope
                .assignment_protocol_digest
                .as_deref()
                .is_some_and(|digest| !digest.trim().is_empty())
                && envelope.identification_assumptions_digest.is_none()
        }
        ResidualEvidenceClass::QualifiedNaturalExperiment => {
            envelope
                .identification_assumptions_digest
                .as_deref()
                .is_some_and(|digest| !digest.trim().is_empty())
                && envelope.assignment_protocol_digest.is_none()
        }
        ResidualEvidenceClass::ObservedFactual
        | ResidualEvidenceClass::SimulationGenerated
        | ResidualEvidenceClass::ModelGenerated
        | ResidualEvidenceClass::FormalReference
        | ResidualEvidenceClass::HumanAdjudication => false,
    };
    if !envelope.rectangular_relaxation
        || !evidence_identification_qualified
        || envelope.scope_digest != manifest.odd_scope_digest
        || envelope.estimand_digest.trim().is_empty()
        || envelope.independent_unit_definition_digest.trim().is_empty()
        || envelope.evidence_digest.trim().is_empty()
        || envelope.calibration_protocol_digest.trim().is_empty()
    {
        return Ok(unbounded());
    }

    let training_lineage: BTreeSet<_> =
        manifest
            .training_lineage_roots
            .iter()
            .map(String::as_str)
            .chain(manifest.hypotheses.iter().flat_map(|hypothesis| {
                hypothesis.source_lineage_digests.iter().map(String::as_str)
            }))
            .collect();
    let holdout_lineage: BTreeSet<_> = challenge_receipts
        .iter()
        .flat_map(|receipt| {
            receipt
                .holdout_parent_lineage_digests
                .iter()
                .map(String::as_str)
        })
        .collect();

    let mut bounds = BTreeMap::new();
    for bound in &envelope.action_bounds {
        require(&bound.action_id, "residual.action_id")?;
        if bound.evidence_digest.trim().is_empty()
            || bound.calibration_protocol_digest.trim().is_empty()
            || bound.source_lineage_digests.is_empty()
            || bound.minimum_independent_units == 0
        {
            return Ok(unbounded());
        }
        if bound.independent_unit_count < bound.minimum_independent_units {
            return Err(PortfolioError::InsufficientResidualUnits);
        }
        for lineage in &bound.source_lineage_digests {
            require(lineage, "residual.source_lineage_digest")?;
            if training_lineage.contains(lineage.as_str())
                || holdout_lineage.contains(lineage.as_str())
            {
                return Err(PortfolioError::ResidualEvidenceLineageLeak);
            }
        }
        if bound.utility_lower.cmp_exact(bound.utility_upper)? == std::cmp::Ordering::Greater
            || bounds.insert(bound.action_id.as_str(), bound).is_some()
        {
            return Ok(unbounded());
        }
    }
    let declared: BTreeSet<_> = manifest.action_ids.iter().map(String::as_str).collect();
    if bounds.keys().copied().collect::<BTreeSet<_>>() != declared {
        return Ok(unbounded());
    }

    let mut rival_upper = Rational::zero();
    let mut first = true;
    for bound in bounds.values() {
        if first {
            rival_upper = bound.utility_upper;
            first = false;
        } else {
            rival_upper = rival_upper.max(bound.utility_upper)?;
        }
    }

    let mut action_regret_bounds = Vec::with_capacity(bounds.len());
    let mut robust_feasible_actions = Vec::new();
    let mut best_regret: Option<Rational> = None;
    for action_id in &manifest.action_ids {
        let bound = bounds[action_id.as_str()];
        let regret = Rational::checked_regret_upper(rival_upper, bound.utility_lower)?;
        action_regret_bounds.push(ActionRegretBound {
            action_id: action_id.clone(),
            worst_case_regret_upper: regret,
            feasibility: bound.feasibility,
        });
        if bound.feasibility == FeasibilityDisposition::Guaranteed {
            robust_feasible_actions.push(action_id.clone());
            best_regret = Some(match best_regret {
                None => regret,
                Some(current) => current.min(regret)?,
            });
        }
    }

    let mut minimax_regret_actions = Vec::new();
    if let Some(best) = best_regret {
        for bound in &action_regret_bounds {
            if bound.feasibility == FeasibilityDisposition::Guaranteed
                && bound.worst_case_regret_upper.cmp_exact(best)? == std::cmp::Ordering::Equal
            {
                minimax_regret_actions.push(bound.action_id.clone());
            }
        }
    }

    Ok(ResidualAssessment {
        disposition: ResidualDisposition::BoundedRectangularRelaxation,
        action_regret_bounds,
        robust_feasible_actions,
        minimax_regret_actions,
        rectangular_bound_is_attainable_joint_world: false,
    })
}

fn hash_hypothesis(h: &mut blake3::Hasher, value: &Hypothesis) {
    hash_str(h, &value.hypothesis_id);
    hash_str(h, &value.model_family_id);
    hash_str(h, &value.version_digest);
    let mut mechanism_classes = value.mechanism_class_ids.clone();
    mechanism_classes.sort();
    hash_strings(h, &mechanism_classes);
    hash_str(h, &value.causal_graph_digest);
    hash_str(h, &value.stochastic_contract_digest);
    let mut lineage = value.source_lineage_digests.clone();
    lineage.sort();
    hash_strings(h, &lineage);
    hash_opt_str(h, value.common_mode_group.as_deref());
    hash_str(h, &value.applicability_scope_digest);
    let mut excluded = value.excluded_mechanisms.clone();
    excluded.sort_by(|a, b| a.mechanism_id.cmp(&b.mechanism_id));
    hash_len(h, excluded.len());
    for mechanism in excluded {
        hash_str(h, &mechanism.mechanism_id);
        hash_str(h, &mechanism.rationale);
    }
    hash_opt_str(h, value.supersedes_hypothesis_id.as_deref());
}

fn hash_coverage(h: &mut blake3::Hasher, value: &CoverageObligation) {
    hash_str(h, &value.obligation_id);
    hash_u8(
        h,
        match value.axis {
            CoverageAxis::OddRange => 0,
            CoverageAxis::BehaviouralCompetency => 1,
            CoverageAxis::CausalMechanism => 2,
            CoverageAxis::RuleSafety => 3,
        },
    );
    hash_str(h, &value.stratum_digest);
    hash_u8(
        h,
        match value.state {
            CoverageState::Covered => 0,
            CoverageState::NotCovered => 1,
            CoverageState::Unknown => 2,
            CoverageState::NotApplicable => 3,
        },
    );
    hash_u64(h, value.minimum_independent_units);
    hash_u64(h, value.independent_unit_count);
    hash_u8(h, value.evidence_class.map(evidence_tag).unwrap_or(u8::MAX));
    hash_opt_str(h, value.evidence_manifest_digest.as_deref());
    hash_opt_str(h, value.source_lineage_digest.as_deref());
    hash_opt_str(h, value.challenge_protocol_digest.as_deref());
    hash_opt_str(h, value.not_applicable_rationale.as_deref());
}

fn evidence_tag(value: EvidenceClass) -> u8 {
    match value {
        EvidenceClass::ObservedFactual => 0,
        EvidenceClass::ControlledIntervention => 1,
        EvidenceClass::QualifiedNaturalExperiment => 2,
        EvidenceClass::SimulationGenerated => 3,
        EvidenceClass::ModelGenerated => 4,
        EvidenceClass::FormalReference => 5,
        EvidenceClass::HumanAdjudication => 6,
    }
}

fn strategy_tag(value: ChallengeStrategy) -> u8 {
    match value {
        ChallengeStrategy::Random => 0,
        ChallengeStrategy::CoverageGuided => 1,
        ChallengeStrategy::Expert => 2,
        ChallengeStrategy::ReinforcementLearning => 3,
    }
}

fn require(value: &str, field: &'static str) -> Result<(), PortfolioError> {
    if value.trim().is_empty() {
        Err(PortfolioError::EmptyIdentity(field))
    } else {
        Ok(())
    }
}

fn hash_str(h: &mut blake3::Hasher, value: &str) {
    hash_len(h, value.len());
    h.update(value.as_bytes());
}

fn hash_opt_str(h: &mut blake3::Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            h.update(&[1]);
            hash_str(h, value);
        }
        None => {
            h.update(&[0]);
        }
    }
}

fn hash_strings(h: &mut blake3::Hasher, values: &[String]) {
    hash_len(h, values.len());
    for value in values {
        hash_str(h, value);
    }
}

fn hash_u8(h: &mut blake3::Hasher, value: u8) {
    h.update(&[value]);
}

fn hash_u64(h: &mut blake3::Hasher, value: u64) {
    h.update(&value.to_le_bytes());
}

fn hash_i64(h: &mut blake3::Hasher, value: i64) {
    h.update(&value.to_le_bytes());
}

fn hash_len(h: &mut blake3::Hasher, value: usize) {
    let value = u64::try_from(value).expect("finite manifest length must fit u64");
    h.update(&value.to_le_bytes());
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: &str) -> String {
        format!("sha256:{value}")
    }

    fn hypothesis(
        hypothesis_id: &str,
        model_family: &str,
        lineage: &str,
        common_mode: Option<&str>,
    ) -> Hypothesis {
        Hypothesis {
            hypothesis_id: hypothesis_id.into(),
            model_family_id: model_family.into(),
            version_digest: d(&format!("version-{hypothesis_id}")),
            mechanism_class_ids: vec!["pedestrian-response".into(), "weather-cause".into()],
            causal_graph_digest: d(&format!("graph-{hypothesis_id}")),
            stochastic_contract_digest: d(&format!("stochastic-{hypothesis_id}")),
            source_lineage_digests: vec![d(lineage)],
            common_mode_group: common_mode.map(str::to_owned),
            applicability_scope_digest: d("odd-city-1"),
            excluded_mechanisms: Vec::new(),
            supersedes_hypothesis_id: None,
        }
    }

    fn obligation(id: &str, axis: CoverageAxis) -> CoverageObligation {
        CoverageObligation {
            obligation_id: id.into(),
            axis,
            stratum_digest: d(&format!("stratum-{id}")),
            state: CoverageState::Covered,
            minimum_independent_units: 5,
            independent_unit_count: 5,
            evidence_class: Some(EvidenceClass::ControlledIntervention),
            evidence_manifest_digest: Some(d(&format!("evidence-{id}"))),
            source_lineage_digest: Some(d(&format!("lineage-{id}"))),
            challenge_protocol_digest: Some(d("challenge-protocol-v1")),
            not_applicable_rationale: None,
        }
    }

    fn manifest() -> PortfolioManifest {
        PortfolioManifest {
            schema: PORTFOLIO_SCHEMA.into(),
            portfolio_id: "road-portfolio-001".into(),
            revision_sequence: 1,
            supersedes_portfolio_digest: None,
            odd_scope_digest: d("odd-city-1"),
            utility_scale_id: "utility-0-to-100".into(),
            utility_semantics_id: "higher-is-better-v1".into(),
            training_lineage_roots: vec![d("training-root")],
            action_ids: vec!["continue".into(), "yield".into()],
            hypotheses: vec![
                hypothesis(
                    "H-weather",
                    "structural-scm",
                    "shared-training",
                    Some("weather-common"),
                ),
                hypothesis(
                    "H-weather-copy",
                    "learned-twin",
                    "shared-training",
                    Some("weather-common"),
                ),
            ],
            coverage_obligations: vec![
                obligation("ODD-rain-night-occluded", CoverageAxis::OddRange),
                obligation("yielding", CoverageAxis::BehaviouralCompetency),
                obligation("shared-weather-descendants", CoverageAxis::CausalMechanism),
                obligation("road-rule-crossing", CoverageAxis::RuleSafety),
            ],
            challenge_requirements: vec![
                ChallengeRequirement {
                    hypothesis_id: "H-weather".into(),
                    strategy: ChallengeStrategy::Random,
                    protocol_digest: d("challenge-random"),
                    minimum_budget_units: 100,
                },
                ChallengeRequirement {
                    hypothesis_id: "H-weather".into(),
                    strategy: ChallengeStrategy::CoverageGuided,
                    protocol_digest: d("challenge-coverage"),
                    minimum_budget_units: 100,
                },
                ChallengeRequirement {
                    hypothesis_id: "H-weather".into(),
                    strategy: ChallengeStrategy::Expert,
                    protocol_digest: d("challenge-expert"),
                    minimum_budget_units: 100,
                },
                ChallengeRequirement {
                    hypothesis_id: "H-weather".into(),
                    strategy: ChallengeStrategy::ReinforcementLearning,
                    protocol_digest: d("challenge-rl"),
                    minimum_budget_units: 100,
                },
                ChallengeRequirement {
                    hypothesis_id: "H-weather-copy".into(),
                    strategy: ChallengeStrategy::Random,
                    protocol_digest: d("challenge-random-copy"),
                    minimum_budget_units: 100,
                },
                ChallengeRequirement {
                    hypothesis_id: "H-weather-copy".into(),
                    strategy: ChallengeStrategy::CoverageGuided,
                    protocol_digest: d("challenge-coverage-copy"),
                    minimum_budget_units: 100,
                },
                ChallengeRequirement {
                    hypothesis_id: "H-weather-copy".into(),
                    strategy: ChallengeStrategy::Expert,
                    protocol_digest: d("challenge-expert-copy"),
                    minimum_budget_units: 100,
                },
                ChallengeRequirement {
                    hypothesis_id: "H-weather-copy".into(),
                    strategy: ChallengeStrategy::ReinforcementLearning,
                    protocol_digest: d("challenge-rl-copy"),
                    minimum_budget_units: 100,
                },
            ],
        }
    }

    fn challenge(
        id: &str,
        strategy: ChallengeStrategy,
        protocol: &str,
        outcome: ChallengeOutcome,
    ) -> ChallengeReceipt {
        let found = outcome == ChallengeOutcome::CounterexampleFound;
        ChallengeReceipt {
            challenge_id: id.into(),
            hypothesis_id: "H-weather".into(),
            strategy,
            protocol_digest: d(protocol),
            holdout_manifest_digest: d("sealed-holdout-v1"),
            blinded: true,
            sealed_test: true,
            budget_units: 100,
            holdout_parent_lineage_digests: vec![d("sealed-holdout-parent-v1")],
            outcome,
            counterexample_digest: found.then_some(d(&format!("counterexample-{id}"))),
            replay_receipt_digest: found.then_some(d(&format!("replay-{id}"))),
            counterexample_replay_verified: found,
        }
    }

    fn challenge_set() -> Vec<ChallengeReceipt> {
        let mut receipts = vec![
            challenge(
                "random-1",
                ChallengeStrategy::Random,
                "challenge-random",
                ChallengeOutcome::NonDiscoveryWithinBudget,
            ),
            challenge(
                "coverage-1",
                ChallengeStrategy::CoverageGuided,
                "challenge-coverage",
                ChallengeOutcome::NonDiscoveryWithinBudget,
            ),
            challenge(
                "expert-1",
                ChallengeStrategy::Expert,
                "challenge-expert",
                ChallengeOutcome::NonDiscoveryWithinBudget,
            ),
            challenge(
                "rl-1",
                ChallengeStrategy::ReinforcementLearning,
                "challenge-rl",
                ChallengeOutcome::NonDiscoveryWithinBudget,
            ),
        ];
        for receipt in &mut receipts {
            receipt.holdout_manifest_digest = d("sealed-holdout-v1");
            receipt.holdout_parent_lineage_digests = vec![d("sealed-holdout-parent-v1")];
        }
        for (id, strategy, protocol) in [
            (
                "random-copy",
                ChallengeStrategy::Random,
                "challenge-random-copy",
            ),
            (
                "coverage-copy",
                ChallengeStrategy::CoverageGuided,
                "challenge-coverage-copy",
            ),
            (
                "expert-copy",
                ChallengeStrategy::Expert,
                "challenge-expert-copy",
            ),
            (
                "rl-copy",
                ChallengeStrategy::ReinforcementLearning,
                "challenge-rl-copy",
            ),
        ] {
            let mut receipt = challenge(
                id,
                strategy,
                protocol,
                ChallengeOutcome::NonDiscoveryWithinBudget,
            );
            receipt.hypothesis_id = "H-weather-copy".into();
            receipt.holdout_manifest_digest = d("sealed-holdout-v1");
            receipt.holdout_parent_lineage_digests = vec![d("sealed-holdout-parent-v1")];
            receipts.push(receipt);
        }
        receipts
    }

    fn residual() -> ResidualEnvelope {
        ResidualEnvelope {
            utility_scale_id: "utility-0-to-100".into(),
            utility_semantics_id: "higher-is-better-v1".into(),
            scope_digest: d("odd-city-1"),
            estimand_digest: d("branch-specific-action-utility-v1"),
            independent_unit_definition_digest: d("independent-drive-episode-v1"),
            evidence_class: ResidualEvidenceClass::ControlledIntervention,
            assignment_protocol_digest: Some(d("randomized-safe-intervention-v1")),
            identification_assumptions_digest: None,
            evidence_digest: d("residual-evidence"),
            calibration_protocol_digest: d("residual-calibration"),
            rectangular_relaxation: true,
            action_bounds: vec![
                ActionResidualBound {
                    action_id: "continue".into(),
                    utility_lower: Rational::new(40, 1).unwrap(),
                    utility_upper: Rational::new(60, 1).unwrap(),
                    evidence_digest: d("continue-residual-evidence"),
                    calibration_protocol_digest: d("continue-calibration"),
                    source_lineage_digests: vec![d("independent-intervention-evidence")],
                    minimum_independent_units: 20,
                    independent_unit_count: 24,
                    feasibility: FeasibilityDisposition::Guaranteed,
                },
                ActionResidualBound {
                    action_id: "yield".into(),
                    utility_lower: Rational::new(50, 1).unwrap(),
                    utility_upper: Rational::new(80, 1).unwrap(),
                    evidence_digest: d("yield-residual-evidence"),
                    calibration_protocol_digest: d("yield-calibration"),
                    source_lineage_digests: vec![d("independent-intervention-evidence")],
                    minimum_independent_units: 20,
                    independent_unit_count: 24,
                    feasibility: FeasibilityDisposition::Guaranteed,
                },
            ],
        }
    }

    #[test]
    fn partially_shared_lineage_is_transitively_collapsed() {
        let hypotheses = vec![
            hypothesis("H-A", "model-a", "source-a", None),
            hypothesis("H-B", "model-b", "source-b", None),
            hypothesis("H-C", "model-c", "source-c", None),
        ];
        let mut hypotheses = hypotheses;
        hypotheses[1].source_lineage_digests.push(d("source-a"));
        hypotheses[2].source_lineage_digests.push(d("source-b"));
        assert_eq!(independent_hypothesis_families(&hypotheses).len(), 1);
    }

    #[test]
    fn distinct_lineage_families_are_not_collapsed() {
        let hypotheses = vec![
            hypothesis("H-A", "model-a", "source-a", None),
            hypothesis("H-B", "model-b", "source-b", None),
        ];
        assert_eq!(independent_hypothesis_families(&hypotheses).len(), 2);
    }

    #[test]
    fn falsification_methods_must_share_the_same_sealed_holdout() {
        let mut receipts = challenge_set();
        receipts[0].holdout_manifest_digest = d("different-holdout");
        assert_eq!(
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &receipts, Some(&residual()),),
            Err(PortfolioError::ChallengeHoldoutMismatch)
        );
    }

    #[test]
    fn falsification_methods_must_use_equal_search_budgets() {
        let mut receipts = challenge_set();
        receipts[0].budget_units += 1;
        assert_eq!(
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &receipts, Some(&residual()),),
            Err(PortfolioError::ChallengeBudgetMismatch)
        );
    }

    #[test]
    fn counterexample_without_verified_replay_cannot_falsify() {
        let mut receipts = challenge_set();
        receipts[0] = challenge(
            "unreplayed-counterexample",
            ChallengeStrategy::Random,
            "challenge-random",
            ChallengeOutcome::CounterexampleFound,
        );
        receipts[0].counterexample_replay_verified = false;
        assert_eq!(
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &receipts, Some(&residual()),),
            Err(PortfolioError::CounterexampleMissingReplay)
        );
    }

    #[test]
    fn residual_evidence_cannot_reuse_training_or_sealed_holdout_lineage() {
        let mut envelope = residual();
        envelope.action_bounds[0].source_lineage_digests = vec![d("training-root")];
        assert_eq!(
            evaluate_portfolio(
                &manifest(),
                &d("odd-city-1"),
                &challenge_set(),
                Some(&envelope),
            ),
            Err(PortfolioError::ResidualEvidenceLineageLeak)
        );

        let mut envelope = residual();
        envelope.action_bounds[0].source_lineage_digests = vec![d("sealed-holdout-parent-v1")];
        assert_eq!(
            evaluate_portfolio(
                &manifest(),
                &d("odd-city-1"),
                &challenge_set(),
                Some(&envelope),
            ),
            Err(PortfolioError::ResidualEvidenceLineageLeak)
        );
    }

    #[test]
    fn residual_evidence_requires_declared_independent_unit_floor() {
        let mut envelope = residual();
        envelope.action_bounds[0].independent_unit_count = 19;
        assert_eq!(
            evaluate_portfolio(
                &manifest(),
                &d("odd-city-1"),
                &challenge_set(),
                Some(&envelope),
            ),
            Err(PortfolioError::InsufficientResidualUnits)
        );
    }

    #[test]
    fn evaluation_digest_binds_challenge_and_residual_inputs() {
        let baseline = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();

        let mut altered_challenges = challenge_set();
        // Keep the entire comparison fair: all arms use the same changed budget.
        for receipt in &mut altered_challenges {
            receipt.budget_units += 1;
        }
        let altered_challenge_report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &altered_challenges,
            Some(&residual()),
        )
        .unwrap();
        assert_ne!(
            baseline.evaluation_digest,
            altered_challenge_report.evaluation_digest
        );

        let mut altered_residual = residual();
        altered_residual.action_bounds[0].independent_unit_count += 1;
        let altered_residual_report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&altered_residual),
        )
        .unwrap();
        assert_ne!(
            baseline.evaluation_digest,
            altered_residual_report.evaluation_digest
        );
    }

    #[test]
    fn complete_portfolio_requires_all_axes_challenges_and_bounded_residuals() {
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        assert_eq!(
            report.disposition,
            PortfolioDisposition::PortfolioCompleteForDeclaredScope
        );
        assert_eq!(report.effective_independent_hypothesis_family_count, 1);
        assert_eq!(report.model_family_count, 2);
        assert!(report.decision_recommendation_authorized);
        assert!(!report.completeness_relative_to_reality_proven);
        assert!(!report.challenge.non_discovery_proves_absence);
    }

    #[test]
    fn cloned_models_in_one_common_mode_group_count_once() {
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        assert_eq!(report.model_family_count, 2);
        assert_eq!(report.effective_independent_hypothesis_family_count, 1);
    }

    #[test]
    fn missing_interaction_combination_is_not_covered_by_attribute_marginals() {
        let mut portfolio = manifest();
        portfolio.coverage_obligations.push(CoverageObligation {
            obligation_id: "rain-x-night-x-occluded".into(),
            axis: CoverageAxis::OddRange,
            stratum_digest: d("rain-night-occluded-crossing"),
            state: CoverageState::NotCovered,
            minimum_independent_units: 10,
            independent_unit_count: 0,
            evidence_class: None,
            evidence_manifest_digest: None,
            source_lineage_digest: None,
            challenge_protocol_digest: None,
            not_applicable_rationale: None,
        });
        let report = evaluate_portfolio(
            &portfolio,
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        assert_eq!(
            report.disposition,
            PortfolioDisposition::PortfolioIncomplete
        );
        assert!(!report.decision_recommendation_authorized);
    }

    #[test]
    fn unknown_coverage_is_not_silently_promoted_to_covered() {
        let mut portfolio = manifest();
        portfolio.coverage_obligations[0].state = CoverageState::Unknown;
        portfolio.coverage_obligations[0].evidence_manifest_digest = None;
        portfolio.coverage_obligations[0].source_lineage_digest = None;
        portfolio.coverage_obligations[0].challenge_protocol_digest = None;
        let report = evaluate_portfolio(
            &portfolio,
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        assert_eq!(
            report.disposition,
            PortfolioDisposition::PortfolioIncomplete
        );
    }

    #[test]
    fn not_applicable_requires_explicit_scope_rationale() {
        let mut portfolio = manifest();
        let obligation = &mut portfolio.coverage_obligations[0];
        obligation.state = CoverageState::NotApplicable;
        obligation.not_applicable_rationale = Some(
            "The declared limited-access-highway-only claim excludes urban crossing approaches."
                .into(),
        );
        obligation.evidence_class = None;
        obligation.evidence_manifest_digest = None;
        obligation.source_lineage_digest = None;
        obligation.challenge_protocol_digest = None;
        let report = evaluate_portfolio(
            &portfolio,
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        assert_eq!(
            report.disposition,
            PortfolioDisposition::PortfolioCompleteForDeclaredScope
        );
    }

    #[test]
    fn budgeted_non_discovery_is_not_a_proof_of_absence() {
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        assert!(!report.challenge.non_discovery_proves_absence);
        assert_eq!(report.challenge.qualified_challenges, 8);
        assert_eq!(report.challenge.unresolved_challenges, 0);
        assert!(
            report
                .challenge
                .strategies
                .iter()
                .all(|entry| entry.attempts == 2)
        );
    }

    #[test]
    fn blinded_replayable_counterexample_falsifies_the_portfolio() {
        let mut receipts = challenge_set();
        receipts[1] = challenge(
            "coverage-counterexample",
            ChallengeStrategy::CoverageGuided,
            "challenge-coverage",
            ChallengeOutcome::CounterexampleFound,
        );
        let report =
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &receipts, Some(&residual()))
                .unwrap();
        assert_eq!(report.disposition, PortfolioDisposition::PortfolioFalsified);
        assert!(!report.decision_recommendation_authorized);
    }

    #[test]
    fn challenge_with_training_parent_leakage_is_rejected() {
        let mut receipts = challenge_set();
        receipts[0].holdout_parent_lineage_digests = vec![d("training-root")];
        assert_eq!(
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &receipts, Some(&residual()),),
            Err(PortfolioError::ChallengeLineageLeak)
        );
    }

    #[test]
    fn non_blinded_challenge_cannot_qualify_a_holdout() {
        let mut receipts = challenge_set();
        receipts[0].blinded = false;
        assert_eq!(
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &receipts, Some(&residual()),),
            Err(PortfolioError::ChallengeNotBlinded)
        );
    }

    #[test]
    fn missing_challenge_or_insufficient_search_budget_keeps_portfolio_incomplete() {
        let mut receipts = challenge_set();
        receipts.pop();
        let report =
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &receipts, Some(&residual()))
                .unwrap();
        assert_eq!(
            report.disposition,
            PortfolioDisposition::PortfolioIncomplete
        );

        let mut low_budget = challenge_set();
        for receipt in &mut low_budget {
            receipt.budget_units = 1;
        }
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &low_budget,
            Some(&residual()),
        )
        .unwrap();
        assert_eq!(
            report.disposition,
            PortfolioDisposition::PortfolioIncomplete
        );
    }

    #[test]
    fn no_empirical_residual_bound_means_unbounded_risk() {
        let report =
            evaluate_portfolio(&manifest(), &d("odd-city-1"), &challenge_set(), None).unwrap();
        assert_eq!(
            report.disposition,
            PortfolioDisposition::ResidualRiskUnbounded
        );
        assert!(!report.decision_recommendation_authorized);
    }

    #[test]
    fn model_dependent_factual_or_uncalibrated_residual_is_unbounded() {
        for evidence_class in [
            ResidualEvidenceClass::ObservedFactual,
            ResidualEvidenceClass::SimulationGenerated,
            ResidualEvidenceClass::ModelGenerated,
            ResidualEvidenceClass::FormalReference,
            ResidualEvidenceClass::HumanAdjudication,
        ] {
            let mut envelope = residual();
            envelope.evidence_class = evidence_class;
            let report = evaluate_portfolio(
                &manifest(),
                &d("odd-city-1"),
                &challenge_set(),
                Some(&envelope),
            )
            .unwrap();
            assert_eq!(
                report.residual.disposition,
                ResidualDisposition::ResidualRiskUnbounded
            );
        }

        let mut envelope = residual();
        envelope.calibration_protocol_digest.clear();
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&envelope),
        )
        .unwrap();
        assert_eq!(
            report.residual.disposition,
            ResidualDisposition::ResidualRiskUnbounded
        );

        let mut envelope = residual();
        envelope.scope_digest = d("different-odd");
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&envelope),
        )
        .unwrap();
        assert_eq!(
            report.residual.disposition,
            ResidualDisposition::ResidualRiskUnbounded
        );
    }

    #[test]
    fn natural_experiment_residual_needs_identification_assumptions() {
        let mut envelope = residual();
        envelope.evidence_class = ResidualEvidenceClass::QualifiedNaturalExperiment;
        envelope.assignment_protocol_digest = None;
        envelope.identification_assumptions_digest = Some(d("natural-experiment-dag-overlap-sensitivity"));
        let qualified = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&envelope),
        )
        .unwrap();
        assert_eq!(
            qualified.residual.disposition,
            ResidualDisposition::BoundedRectangularRelaxation
        );

        envelope.identification_assumptions_digest = None;
        let unqualified = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&envelope),
        )
        .unwrap();
        assert_eq!(
            unqualified.residual.disposition,
            ResidualDisposition::ResidualRiskUnbounded
        );
    }

    #[test]
    fn rectangular_residual_regret_is_exact_and_labeled_as_relaxation() {
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        let continue_bound = report
            .residual
            .action_regret_bounds
            .iter()
            .find(|bound| bound.action_id == "continue")
            .unwrap();
        let yield_bound = report
            .residual
            .action_regret_bounds
            .iter()
            .find(|bound| bound.action_id == "yield")
            .unwrap();

        // max U(b)=80. Regret upper: continue 80-40=40; yield 80-50=30.
        assert_eq!(
            continue_bound.worst_case_regret_upper,
            Rational::new(40, 1).unwrap()
        );
        assert_eq!(
            yield_bound.worst_case_regret_upper,
            Rational::new(30, 1).unwrap()
        );
        assert_eq!(
            report.residual.minimax_regret_actions,
            vec!["yield".to_owned()]
        );
        assert!(!report.residual.rectangular_bound_is_attainable_joint_world);
    }

    #[test]
    fn unknown_or_potential_violation_excludes_an_action_from_robust_set() {
        let mut envelope = residual();
        envelope.action_bounds[0].feasibility = FeasibilityDisposition::Unknown;
        envelope.action_bounds[1].feasibility = FeasibilityDisposition::PotentialViolation;
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&envelope),
        )
        .unwrap();
        assert!(report.residual.robust_feasible_actions.is_empty());
        assert!(!report.decision_recommendation_authorized);
    }

    #[test]
    fn missing_action_bound_keeps_residual_risk_unbounded() {
        let mut envelope = residual();
        envelope.action_bounds.pop();
        let report = evaluate_portfolio(
            &manifest(),
            &d("odd-city-1"),
            &challenge_set(),
            Some(&envelope),
        )
        .unwrap();
        assert_eq!(
            report.residual.disposition,
            ResidualDisposition::ResidualRiskUnbounded
        );
    }

    #[test]
    fn out_of_scope_is_not_a_successful_portfolio_evaluation() {
        let report = evaluate_portfolio(
            &manifest(),
            &d("different-odd"),
            &challenge_set(),
            Some(&residual()),
        )
        .unwrap();
        assert_eq!(report.disposition, PortfolioDisposition::OutOfScope);
        assert!(!report.decision_recommendation_authorized);
    }

    #[test]
    fn append_only_revision_rejects_mutating_an_existing_challenged_hypothesis() {
        let previous = manifest();
        let mut current = previous.clone();
        current.revision_sequence = 2;
        current.supersedes_portfolio_digest = Some(previous.portfolio_digest().unwrap());
        current.hypotheses[0].version_digest = d("silently-replaced-version");
        assert_eq!(
            PortfolioManifest::validate_append_only_revision(&previous, &current),
            Err(PortfolioError::NonAppendOnlyRevision)
        );
    }

    #[test]
    fn append_only_revision_accepts_addition_under_new_hypothesis_identity() {
        let previous = manifest();
        let mut current = previous.clone();
        current.revision_sequence = 2;
        current.supersedes_portfolio_digest = Some(previous.portfolio_digest().unwrap());
        let mut added = hypothesis("H-weather-v2", "structural-scm", "new-training", None);
        added.supersedes_hypothesis_id = Some("H-weather".into());
        current.hypotheses.push(added);
        for (strategy, protocol) in [
            (ChallengeStrategy::Random, "challenge-v2-random"),
            (ChallengeStrategy::CoverageGuided, "challenge-v2-coverage"),
            (ChallengeStrategy::Expert, "challenge-v2-expert"),
            (ChallengeStrategy::ReinforcementLearning, "challenge-v2-rl"),
        ] {
            current.challenge_requirements.push(ChallengeRequirement {
                hypothesis_id: "H-weather-v2".into(),
                strategy,
                protocol_digest: d(protocol),
                minimum_budget_units: 100,
            });
        }
        PortfolioManifest::validate_append_only_revision(&previous, &current).unwrap();
    }

    #[test]
    fn append_only_revision_cannot_remove_prior_coverage_or_challenge_obligations() {
        let previous = manifest();

        let mut changed_coverage = previous.clone();
        changed_coverage.revision_sequence = 2;
        changed_coverage.supersedes_portfolio_digest = Some(previous.portfolio_digest().unwrap());
        changed_coverage
            .coverage_obligations
            .push(obligation("extra-odd-stratum", CoverageAxis::OddRange));
        changed_coverage.coverage_obligations.push(obligation(
            "extra-behaviour",
            CoverageAxis::BehaviouralCompetency,
        ));
        changed_coverage
            .coverage_obligations
            .retain(|entry| entry.obligation_id != "yielding");
        assert_eq!(
            PortfolioManifest::validate_append_only_revision(&previous, &changed_coverage),
            Err(PortfolioError::NonAppendOnlyRevision)
        );

        let mut changed_challenges = previous.clone();
        changed_challenges.revision_sequence = 2;
        changed_challenges.supersedes_portfolio_digest = Some(previous.portfolio_digest().unwrap());
        changed_challenges
            .challenge_requirements
            .push(ChallengeRequirement {
                hypothesis_id: "H-weather".into(),
                strategy: ChallengeStrategy::Random,
                protocol_digest: d("extra-random-protocol"),
                minimum_budget_units: 100,
            });
        changed_challenges
            .challenge_requirements
            .push(ChallengeRequirement {
                hypothesis_id: "H-weather-copy".into(),
                strategy: ChallengeStrategy::Expert,
                protocol_digest: d("replacement-expert-protocol"),
                minimum_budget_units: 100,
            });
        changed_challenges.challenge_requirements.retain(|entry| {
            !(entry.hypothesis_id == "H-weather-copy"
                && entry.strategy == ChallengeStrategy::Expert
                && entry.protocol_digest == d("challenge-expert-copy"))
        });
        assert_eq!(
            PortfolioManifest::validate_append_only_revision(&previous, &changed_challenges),
            Err(PortfolioError::NonAppendOnlyRevision)
        );
    }

    #[test]
    fn rational_arithmetic_normalizes_and_rejects_zero_denominator() {
        assert_eq!(Rational::new(6, 8).unwrap(), Rational::new(3, 4).unwrap());
        assert_eq!(Rational::new(1, 0), Err(RationalError::ZeroDenominator));
        assert_eq!(
            Rational::checked_regret_upper(
                Rational::new(2, 3).unwrap(),
                Rational::new(3, 4).unwrap(),
            )
            .unwrap(),
            Rational::zero()
        );
    }
}
