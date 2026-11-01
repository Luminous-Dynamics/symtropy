//! Append-only, split-aware admission and evaluation ledger for counterexamples.
//!
//! This module governs how generated counterexamples may enter a finite
//! development family, forces a complete shared-policy replay after expansion,
//! and makes sealed holdout consumption one-way. Policy and artifact identities
//! are opaque caller-supplied identifiers, not cryptographic hashes.

use super::counterexample_search::{
    CounterexampleClass, CounterexampleOutcome, CounterexampleWitness, ModelWorldPerturbation,
    apply_model_world_perturbation, is_policy_invalidated_error,
};
use super::finite_horizon::{DecisionBudgets, DecisionPlan};
use super::robust_finite_horizon::{
    DecisionModelWorld, WorldDecisionObjectives, replay_shared_policy_across_worlds,
};
use super::{DecisionLossTable, ObservationProtocol};
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_LEDGER_WORLDS: usize = 16;
const MAX_LEDGER_EVENTS: usize = 100_000;
const MAX_SEALED_GROUPS: usize = 256;
const MAX_WORLD_ID_LENGTH: usize = 256;
const MAX_RECEIPT_ID_LENGTH: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EvidenceSplit {
    Development,
    Calibration,
    SealedHoldout,
}

/// Immutable exact-replay context shared by initial, development-family and
/// sealed-holdout evaluations. A family ledger cannot silently switch loss
/// semantics, telemetry protocol, action costs or horizon between revisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionReplayContext {
    pub nominal_world_id: String,
    pub loss_table: DecisionLossTable,
    pub protocol: ObservationProtocol,
    pub command_costs: Vec<Rational>,
    pub hazard_loss: Rational,
    pub horizon: usize,
    pub development_budgets: DecisionBudgets,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleProvenance {
    pub split: EvidenceSplit,
    pub family_group_id: String,
    pub generator_id: String,
    pub generator_version: String,
    pub search_run_id: String,
    pub seed: u64,
    pub family_revision: u64,
    pub policy_identity: String,
    pub attacked_policy: DecisionPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleValidationReceipt {
    pub receipt_id: String,
    pub validator_id: String,
    pub validator_version: String,
    pub evidence_artifact_id: String,
    pub schema_valid: bool,
    pub physical_plausibility_passed: bool,
    pub independent_provenance_passed: bool,
    pub origin_split_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedHoldoutGroupRegistration {
    pub family_group_id: String,
    pub world_ids: Vec<String>,
    pub manifest_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleAdmissionRecord {
    pub event_receipt_id: String,
    pub revision_before: u64,
    pub revision_after: u64,
    pub source_world_id: String,
    pub source_world_snapshot: DecisionModelWorld,
    pub candidate_world_id: String,
    pub candidate_world_snapshot: DecisionModelWorld,
    pub perturbation: ModelWorldPerturbation,
    pub provenance: CounterexampleProvenance,
    pub validation: CounterexampleValidationReceipt,
    pub invalidated_policy_identity: String,
    pub independently_replayed_outcome: CounterexampleOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyReplanRecord {
    pub event_receipt_id: String,
    pub family_revision: u64,
    pub policy_identity: String,
    pub plan: DecisionPlan,
    pub replayed_world_ids: Vec<String>,
    pub per_world_metrics: Vec<WorldDecisionObjectives>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalPolicySealRecord {
    pub event_receipt_id: String,
    pub family_revision: u64,
    pub policy_identity: String,
    pub plan: DecisionPlan,
    pub per_world_metrics: Vec<WorldDecisionObjectives>,
}

/// Pre-registered numeric acceptance envelope for a sealed evaluation. The
/// final evaluation verdict is computed from exact replay, not caller supplied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SealedHoldoutAcceptanceCriteria {
    pub max_worst_case_total_loss: Rational,
    pub max_worst_case_terminal_loss: Rational,
    pub budgets: DecisionBudgets,
}

impl SealedHoldoutAcceptanceCriteria {
    fn validate(self) -> Result<(), String> {
        // Rational is a reduced non-negative type; only the risk budgets are
        // probabilities and therefore require [0,1] validation.
        self.budgets.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedHoldoutEvaluationRecord {
    pub event_receipt_id: String,
    pub family_group_id: String,
    pub manifest_id: String,
    pub family_revision: u64,
    pub policy_identity: String,
    pub world_ids: Vec<String>,
    pub holdout_world_snapshots: Vec<DecisionModelWorld>,
    pub evaluation_artifact_id: String,
    pub criteria: SealedHoldoutAcceptanceCriteria,
    pub replay_succeeded: bool,
    pub replay_error: Option<String>,
    pub per_world_metrics: Vec<WorldDecisionObjectives>,
    pub worst_case_total_loss: Option<Rational>,
    pub worst_case_terminal_loss: Option<Rational>,
    pub acceptance_criteria_satisfied: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelFamilyLedgerEvent {
    CounterexampleAdmitted(CounterexampleAdmissionRecord),
    PolicyReplanned(FamilyReplanRecord),
    FinalPolicySealed(FinalPolicySealRecord),
    SealedHoldoutEvaluated(SealedHoldoutEvaluationRecord),
}

impl ModelFamilyLedgerEvent {
    pub fn receipt_id(&self) -> &str {
        match self {
            Self::CounterexampleAdmitted(record) => &record.event_receipt_id,
            Self::PolicyReplanned(record) => &record.event_receipt_id,
            Self::FinalPolicySealed(record) => &record.event_receipt_id,
            Self::SealedHoldoutEvaluated(record) => &record.event_receipt_id,
        }
    }

    pub fn family_revision(&self) -> u64 {
        match self {
            Self::CounterexampleAdmitted(record) => record.revision_after,
            Self::PolicyReplanned(record) => record.family_revision,
            Self::FinalPolicySealed(record) => record.family_revision,
            Self::SealedHoldoutEvaluated(record) => record.family_revision,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequencedLedgerEvent {
    pub sequence: u64,
    pub event: ModelFamilyLedgerEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelFamilyLedgerSnapshot {
    pub family_revision: u64,
    pub replay_context: DecisionReplayContext,
    pub development_worlds: Vec<DecisionModelWorld>,
    pub active_policy_identity: Option<String>,
    pub active_policy_plan: Option<DecisionPlan>,
    pub active_policy_metrics: Option<Vec<WorldDecisionObjectives>>,
    pub frozen_policy_identity: Option<String>,
    pub frozen_policy_plan: Option<DecisionPlan>,
    pub frozen_policy_metrics: Option<Vec<WorldDecisionObjectives>>,
    pub frozen_policy_revision: Option<u64>,
    pub reserved_holdout_groups: Vec<SealedHoldoutGroupRegistration>,
    pub consumed_holdout_group_ids: Vec<String>,
    pub used_validation_receipt_ids: Vec<String>,
    pub invalidated_policy_identity: Option<String>,
    pub events: Vec<SequencedLedgerEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelFamilyLedger {
    replay_context: DecisionReplayContext,
    development_worlds: Vec<DecisionModelWorld>,
    family_revision: u64,
    active_policy_identity: Option<String>,
    active_policy_plan: Option<DecisionPlan>,
    active_policy_metrics: Option<Vec<WorldDecisionObjectives>>,
    invalidated_policy_identity: Option<String>,
    frozen_policy_identity: Option<String>,
    frozen_policy_plan: Option<DecisionPlan>,
    frozen_policy_metrics: Option<Vec<WorldDecisionObjectives>>,
    frozen_policy_revision: Option<u64>,
    reserved_holdouts: BTreeMap<String, SealedHoldoutGroupRegistration>,
    consumed_holdout_group_ids: BTreeSet<String>,
    event_receipt_ids: BTreeSet<String>,
    validation_receipt_ids: BTreeSet<String>,
    events: Vec<SequencedLedgerEvent>,
}

impl ModelFamilyLedger {
    /// Initialize the immutable development baseline and reserve sealed groups
    /// before counterexample search starts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        baseline_worlds: Vec<DecisionModelWorld>,
        holdouts: Vec<SealedHoldoutGroupRegistration>,
        initial_policy_identity: String,
        initial_policy_plan: DecisionPlan,
        nominal_world_id: &str,
        loss_table: &DecisionLossTable,
        protocol: ObservationProtocol,
        command_costs: &[Rational],
        hazard_loss: Rational,
        horizon: usize,
        budgets: DecisionBudgets,
    ) -> Result<Self, String> {
        validate_opaque_id(
            &initial_policy_identity,
            MAX_RECEIPT_ID_LENGTH,
            "initial policy identity",
        )?;
        if baseline_worlds.is_empty() || baseline_worlds.len() > MAX_LEDGER_WORLDS {
            return Err(format!(
                "baseline family size must be in 1..={MAX_LEDGER_WORLDS}"
            ));
        }
        budgets.validate()?;
        validate_opaque_id(nominal_world_id, MAX_WORLD_ID_LENGTH, "nominal world ID")?;
        let replay_context = DecisionReplayContext {
            nominal_world_id: nominal_world_id.into(),
            loss_table: loss_table.clone(),
            protocol,
            command_costs: command_costs.to_vec(),
            hazard_loss,
            horizon,
            development_budgets: budgets,
        };

        let mut world_ids = BTreeSet::new();
        for world in &baseline_worlds {
            validate_opaque_id(&world.id, MAX_WORLD_ID_LENGTH, "baseline world ID")?;
            if !world_ids.insert(world.id.as_str()) {
                return Err("baseline world IDs must be unique".into());
            }
            world.kernel.validate()?;
        }
        let initial_replay = replay_shared_policy_across_worlds(
            &baseline_worlds,
            &replay_context.nominal_world_id,
            &replay_context.loss_table,
            replay_context.protocol,
            &replay_context.command_costs,
            replay_context.hazard_loss,
            &initial_policy_plan,
            replay_context.horizon,
        )?;
        if initial_replay.per_world.iter().any(|metrics| {
            metrics.hazard_probability > budgets.max_hazard_probability
                || metrics.abstention_probability > budgets.max_abstention_probability
        }) {
            return Err("initial policy violates hazard or abstention budget in a baseline world".into());
        }

        if holdouts.len() > MAX_SEALED_GROUPS {
            return Err(format!(
                "sealed holdout group count exceeds {MAX_SEALED_GROUPS}"
            ));
        }
        let mut reserved_holdouts = BTreeMap::new();
        let mut sealed_world_ids = BTreeSet::new();
        for registration in holdouts {
            validate_opaque_id(
                &registration.family_group_id,
                MAX_WORLD_ID_LENGTH,
                "sealed family-group ID",
            )?;
            validate_opaque_id(
                &registration.manifest_id,
                MAX_RECEIPT_ID_LENGTH,
                "sealed manifest ID",
            )?;
            if registration.world_ids.is_empty() {
                return Err("sealed holdout registration must contain at least one world".into());
            }
            let mut local_ids = BTreeSet::new();
            for world_id in &registration.world_ids {
                validate_opaque_id(world_id, MAX_WORLD_ID_LENGTH, "sealed world ID")?;
                if !local_ids.insert(world_id.as_str()) {
                    return Err("sealed world IDs must be unique within a group".into());
                }
                if world_ids.contains(world_id.as_str()) {
                    return Err("sealed holdout world ID collides with a development world".into());
                }
                if !sealed_world_ids.insert(world_id.clone()) {
                    return Err("sealed world IDs cannot belong to multiple groups".into());
                }
            }
            let group_id = registration.family_group_id.clone();
            if reserved_holdouts.insert(group_id, registration).is_some() {
                return Err("sealed family-group IDs must be unique".into());
            }
        }

        Ok(Self {
            replay_context,
            development_worlds: baseline_worlds,
            family_revision: 0,
            active_policy_identity: Some(initial_policy_identity),
            active_policy_plan: Some(initial_policy_plan),
            active_policy_metrics: Some(initial_replay.per_world),
            invalidated_policy_identity: None,
            frozen_policy_identity: None,
            frozen_policy_plan: None,
            frozen_policy_metrics: None,
            frozen_policy_revision: None,
            reserved_holdouts,
            consumed_holdout_group_ids: BTreeSet::new(),
            event_receipt_ids: BTreeSet::new(),
            validation_receipt_ids: BTreeSet::new(),
            events: Vec::new(),
        })
    }

    pub fn family_revision(&self) -> u64 {
        self.family_revision
    }

    pub fn active_policy_identity(&self) -> Option<&str> {
        self.active_policy_identity.as_deref()
    }

    pub fn is_policy_frozen(&self) -> bool {
        self.frozen_policy_identity.is_some()
    }

    pub fn development_world_ids(&self) -> Vec<String> {
        let mut ids = self
            .development_worlds
            .iter()
            .map(|world| world.id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    pub fn events(&self) -> &[SequencedLedgerEvent] {
        &self.events
    }

    pub fn snapshot(&self) -> ModelFamilyLedgerSnapshot {
        let mut consumed_holdout_group_ids = self
            .consumed_holdout_group_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        consumed_holdout_group_ids.sort();
        ModelFamilyLedgerSnapshot {
            family_revision: self.family_revision,
            replay_context: self.replay_context.clone(),
            development_worlds: self.development_worlds.clone(),
            active_policy_identity: self.active_policy_identity.clone(),
            active_policy_plan: self.active_policy_plan.clone(),
            active_policy_metrics: self.active_policy_metrics.clone(),
            frozen_policy_identity: self.frozen_policy_identity.clone(),
            frozen_policy_plan: self.frozen_policy_plan.clone(),
            frozen_policy_metrics: self.frozen_policy_metrics.clone(),
            frozen_policy_revision: self.frozen_policy_revision,
            reserved_holdout_groups: self.reserved_holdouts.values().cloned().collect(),
            consumed_holdout_group_ids,
            used_validation_receipt_ids: self.validation_receipt_ids.iter().cloned().collect(),
            invalidated_policy_identity: self.invalidated_policy_identity.clone(),
            events: self.events.clone(),
        }
    }

    /// Admit a validated development counterexample, append its model world,
    /// increment the family revision and invalidate the attacked policy.
    pub fn admit_counterexample(
        &mut self,
        witness: &CounterexampleWitness,
        provenance: CounterexampleProvenance,
        validation: CounterexampleValidationReceipt,
        event_receipt_id: String,
    ) -> Result<CounterexampleAdmissionRecord, String> {
        self.ensure_refinement_open()?;
        self.validate_new_receipt_id(&event_receipt_id)?;
        validate_validation_receipt(&validation)?;
        if self.validation_receipt_ids.contains(&validation.receipt_id) {
            return Err("counterexample validation receipt IDs must be unique".into());
        }
        validate_provenance(&provenance)?;
        if provenance.family_revision != self.family_revision {
            return Err("counterexample provenance is bound to a stale model-family revision".into());
        }

        if provenance.split != EvidenceSplit::Development {
            return Err("only Development-origin counterexamples may expand the model family".into());
        }
        if self.reserved_holdouts.contains_key(&provenance.family_group_id) {
            return Err("development candidate overlaps a reserved sealed holdout group".into());
        }
        if witness.outcome.class() == CounterexampleClass::NoViolation {
            return Err("a NoViolation candidate cannot be admitted as a counterexample".into());
        }
        if witness.spec.candidate_world_id != witness.candidate_world.id {
            return Err("candidate world ID does not match the perturbation specification".into());
        }
        if witness.spec.source_world_id == witness.spec.candidate_world_id {
            return Err("candidate and source world IDs must differ".into());
        }
        let active_policy = self
            .active_policy_identity
            .as_ref()
            .ok_or_else(|| "no active policy exists; a replan is required before another attack".to_string())?;
        if provenance.policy_identity != *active_policy {
            return Err("counterexample was generated against a stale or different policy identity".into());
        }
        let active_plan = self
            .active_policy_plan
            .as_ref()
            .ok_or_else(|| "active policy identity has no retained policy tree".to_string())?;
        if provenance.attacked_policy != *active_plan {
            return Err("counterexample provenance does not contain the exact active policy tree".into());
        }
        let baseline_terminal_loss = self
            .active_policy_metrics
            .as_ref()
            .ok_or_else(|| "active policy identity has no retained replay metrics".to_string())?
            .iter()
            .map(|metrics| metrics.expected_terminal_loss)
            .max()
            .unwrap_or_else(Rational::zero);

        let source = self
            .development_worlds
            .iter()
            .find(|world| world.id == witness.spec.source_world_id)
            .cloned()
            .ok_or_else(|| "counterexample source world is not in the current development family".to_string())?;
        validate_world_identity(&witness.candidate_world)?;
        let reconstructed_candidate = apply_model_world_perturbation(
            &source,
            &witness.candidate_world.id,
            &witness.spec.perturbation,
        )?;
        if reconstructed_candidate != witness.candidate_world {
            return Err("candidate model world does not equal the declared source-plus-perturbation result".into());
        }

        if self
            .development_worlds
            .iter()
            .any(|existing| existing.id == witness.candidate_world.id)
        {
            return Err("candidate world ID already exists in the development family".into());
        }
        if self
            .reserved_holdouts
            .values()
            .any(|registration| registration.world_ids.contains(&witness.candidate_world.id))
        {
            return Err("candidate world ID collides with a pre-registered sealed world ID".into());
        }
        if self
            .development_worlds
            .iter()
            .any(|existing| existing.kernel == witness.candidate_world.kernel)
        {
            return Err("candidate world duplicates existing model content under a new ID".into());
        }
        if self.development_worlds.len() >= MAX_LEDGER_WORLDS {
            return Err(format!(
                "model-family world cap {MAX_LEDGER_WORLDS} would be exceeded"
            ));
        }

        // Independently replay the current policy on the complete family plus
        // the candidate. A witness label is not accepted as proof of violation.
        let mut attacked_worlds = self.development_worlds.clone();
        attacked_worlds.push(witness.candidate_world.clone());
        let replay_outcome = match replay_shared_policy_across_worlds(
            &attacked_worlds,
            &self.replay_context.nominal_world_id,
            &self.replay_context.loss_table,
            self.replay_context.protocol,
            &self.replay_context.command_costs,
            self.replay_context.hazard_loss,
            active_plan,
            self.replay_context.horizon,
        ) {
            Err(reason) if is_policy_invalidated_error(&reason) => {
                CounterexampleOutcome::PolicyInvalidated { reason }
            }
            Err(reason) => {
                return Err(format!("counterexample replay was inconclusive: {reason}"));
            }
            Ok(replay) => {
                let candidate_metrics = replay
                    .per_world
                    .iter()
                    .find(|metrics| metrics.world_id == witness.candidate_world.id)
                    .cloned()
                    .ok_or_else(|| "candidate world is missing from independent replay".to_string())?;
                if candidate_metrics.hazard_probability
                    > self.replay_context.development_budgets.max_hazard_probability
                {
                    CounterexampleOutcome::HazardBudgetViolated {
                        metrics: candidate_metrics,
                        allowed_hazard_probability: self.replay_context.development_budgets.max_hazard_probability,
                    }
                } else if candidate_metrics.abstention_probability
                    > self.replay_context.development_budgets.max_abstention_probability
                {
                    CounterexampleOutcome::AbstentionBudgetViolated {
                        metrics: candidate_metrics,
                        allowed_abstention_probability: self.replay_context.development_budgets.max_abstention_probability,
                    }
                } else if candidate_metrics.expected_terminal_loss > baseline_terminal_loss {
                    CounterexampleOutcome::DecisionLossAmplified {
                        metrics: candidate_metrics,
                        baseline_worst_case_terminal_loss: baseline_terminal_loss,
                    }
                } else {
                    CounterexampleOutcome::NoViolation {
                        metrics: candidate_metrics,
                    }
                }
            }
        };
        if !counterexample_outcomes_match(&witness.outcome, &replay_outcome) {
            return Err("counterexample witness outcome does not match independent ledger replay".into());
        }
        if replay_outcome.class() == CounterexampleClass::NoViolation {
            return Err("independent replay found no policy or decision violation".into());
        }

        if self.events.len() >= MAX_LEDGER_EVENTS {
            return Err("model-family ledger event cap would be exceeded".into());
        }

        let revision_after = self
            .family_revision
            .checked_add(1)
            .ok_or_else(|| "model-family revision overflow".to_string())?;
        let invalidated_policy_identity = active_policy.clone();
        let record = CounterexampleAdmissionRecord {
            event_receipt_id: event_receipt_id.clone(),
            revision_before: self.family_revision,
            revision_after,
            source_world_id: source.id.clone(),
            source_world_snapshot: source,
            candidate_world_id: witness.candidate_world.id.clone(),
            candidate_world_snapshot: witness.candidate_world.clone(),
            perturbation: witness.spec.perturbation.clone(),
            provenance,
            validation,
            invalidated_policy_identity: invalidated_policy_identity.clone(),
            independently_replayed_outcome: replay_outcome,
        };

        self.development_worlds.push(witness.candidate_world.clone());
        self.family_revision = revision_after;
        self.invalidated_policy_identity = Some(invalidated_policy_identity);
        self.active_policy_identity = None;
        self.active_policy_plan = None;
        self.active_policy_metrics = None;
        self.append_event(
            event_receipt_id,
            ModelFamilyLedgerEvent::CounterexampleAdmitted(record.clone()),
        )?;
        self.validation_receipt_ids.insert(record.validation.receipt_id.clone());
        Ok(record)
    }

    /// Reactivate a policy only after exact replay against precisely every
    /// development world at the current family revision.
    /// Reactivate a policy only after the ledger itself has replayed the shared
    /// policy against every world at the current family revision. The caller
    /// supplies the complete replay context; a Boolean success flag is never
    /// accepted as evidence.
    #[allow(clippy::too_many_arguments)]
    /// Reactivate a policy only after the ledger itself has replayed it
    /// against every stored development world under the immutable replay
    /// context established at ledger initialization.
    pub fn record_replan(
        &mut self,
        event_receipt_id: String,
        family_revision: u64,
        policy_identity: String,
        plan: DecisionPlan,
    ) -> Result<FamilyReplanRecord, String> {
        self.ensure_refinement_open()?;
        self.validate_new_receipt_id(&event_receipt_id)?;
        if self.active_policy_identity.is_some() {
            return Err("the current family already has an active policy".into());
        }
        if family_revision != self.family_revision {
            return Err("replan receipt is bound to a stale family revision".into());
        }
        validate_opaque_id(&policy_identity, MAX_RECEIPT_ID_LENGTH, "replanned policy identity")?;
        if self.invalidated_policy_identity.as_ref() == Some(&policy_identity) {
            return Err("a new policy identity is required after counterexample invalidation".into());
        }

        let replay = replay_shared_policy_across_worlds(
            &self.development_worlds,
            &self.replay_context.nominal_world_id,
            &self.replay_context.loss_table,
            self.replay_context.protocol,
            &self.replay_context.command_costs,
            self.replay_context.hazard_loss,
            &plan,
            self.replay_context.horizon,
        )?;
        let replayed_world_ids = replay
            .per_world
            .iter()
            .map(|metrics| metrics.world_id.clone())
            .collect::<Vec<_>>();
        let expected_world_ids = self.development_world_ids();
        let actual_world_ids = canonical_world_id_set(&replayed_world_ids)?;
        if actual_world_ids != expected_world_ids {
            return Err(
                "independent policy replay did not cover exactly the current development family"
                    .into(),
            );
        }
        for metrics in &replay.per_world {
            if metrics.hazard_probability
                > self.replay_context.development_budgets.max_hazard_probability
            {
                return Err(format!(
                    "replanned policy violates hazard budget in world {}",
                    metrics.world_id
                ));
            }
            if metrics.abstention_probability
                > self.replay_context.development_budgets.max_abstention_probability
            {
                return Err(format!(
                    "replanned policy violates abstention budget in world {}",
                    metrics.world_id
                ));
            }
        }
        if self.events.len() >= MAX_LEDGER_EVENTS {
            return Err("model-family ledger event cap would be exceeded".into());
        }

        let record = FamilyReplanRecord {
            event_receipt_id: event_receipt_id.clone(),
            family_revision,
            policy_identity: policy_identity.clone(),
            plan: plan.clone(),
            replayed_world_ids: expected_world_ids,
            per_world_metrics: replay.per_world.clone(),
        };
        self.active_policy_identity = Some(policy_identity);
        self.active_policy_plan = Some(plan);
        self.active_policy_metrics = Some(replay.per_world);
        self.invalidated_policy_identity = None;
        self.append_event(
            event_receipt_id,
            ModelFamilyLedgerEvent::PolicyReplanned(record.clone()),
        )?;
        Ok(record)
    }

    /// Freeze the currently active policy and family revision before opening
    /// any sealed evaluation group.
    pub fn seal_final_policy(
        &mut self,
        event_receipt_id: String,
        family_revision: u64,
        policy_identity: String,
    ) -> Result<FinalPolicySealRecord, String> {
        self.ensure_refinement_open()?;
        self.validate_new_receipt_id(&event_receipt_id)?;
        if self.frozen_policy_identity.is_some() {
            return Err("a final policy has already been sealed".into());
        }
        if family_revision != self.family_revision {
            return Err("seal receipt is bound to a stale family revision".into());
        }
        let active = self
            .active_policy_identity
            .as_ref()
            .ok_or_else(|| "cannot seal while the model family has no active policy".to_string())?;
        if policy_identity != *active {
            return Err("seal policy identity does not match the active policy".into());
        }
        if self.events.len() >= MAX_LEDGER_EVENTS {
            return Err("model-family ledger event cap would be exceeded".into());
        }

        let active_plan = self
            .active_policy_plan
            .as_ref()
            .ok_or_else(|| "active policy identity has no retained policy tree".to_string())?
            .clone();
        let active_metrics = self
            .active_policy_metrics
            .as_ref()
            .ok_or_else(|| "active policy has no retained per-world replay metrics".to_string())?
            .clone();
        let record = FinalPolicySealRecord {
            event_receipt_id: event_receipt_id.clone(),
            family_revision,
            policy_identity: policy_identity.clone(),
            plan: active_plan.clone(),
            per_world_metrics: active_metrics.clone(),
        };
        self.frozen_policy_identity = Some(policy_identity);
        self.frozen_policy_plan = Some(active_plan);
        self.frozen_policy_metrics = Some(active_metrics);
        self.frozen_policy_revision = Some(family_revision);
        self.append_event(
            event_receipt_id,
            ModelFamilyLedgerEvent::FinalPolicySealed(record.clone()),
        )?;
        Ok(record)
    }

    /// Consume one pre-registered sealed holdout group. The ledger itself
    /// replays the frozen final policy against the exact registered worlds and
    /// calculates acceptance from pre-registered numeric criteria. A group is
    /// consumed whether the criteria pass or fail.
    #[allow(clippy::too_many_arguments)]
    /// Consume one pre-registered sealed holdout group. The ledger itself
    /// replays the frozen policy under its immutable context, and calculates
    /// acceptance from pre-registered criteria.
    pub fn record_sealed_holdout_evaluation(
        &mut self,
        event_receipt_id: String,
        family_group_id: String,
        family_revision: u64,
        policy_identity: String,
        holdout_worlds: &[DecisionModelWorld],
        criteria: SealedHoldoutAcceptanceCriteria,
        evaluation_artifact_id: String,
    ) -> Result<SealedHoldoutEvaluationRecord, String> {
        self.validate_new_receipt_id(&event_receipt_id)?;
        criteria.validate()?;
        let frozen_policy = self
            .frozen_policy_identity
            .as_ref()
            .ok_or_else(|| "sealed holdout evaluation requires a frozen final policy".to_string())?;
        let frozen_revision = self
            .frozen_policy_revision
            .ok_or_else(|| "sealed final policy is missing its revision".to_string())?;
        if family_revision != frozen_revision {
            return Err("holdout evaluation does not match the frozen family revision".into());
        }
        if policy_identity != *frozen_policy {
            return Err("holdout evaluation policy identity differs from the frozen policy".into());
        }
        let frozen_plan = self
            .frozen_policy_plan
            .as_ref()
            .ok_or_else(|| "frozen policy identity has no retained policy tree".to_string())?;
        let registration = self
            .reserved_holdouts
            .get(&family_group_id)
            .ok_or_else(|| "holdout family group was not pre-registered".to_string())?;
        if self.consumed_holdout_group_ids.contains(&family_group_id) {
            return Err("sealed holdout group has already been consumed".into());
        }
        let mut expected_world_ids = registration.world_ids.clone();
        expected_world_ids.sort();
        let supplied_world_ids = holdout_worlds
            .iter()
            .map(|world| world.id.clone())
            .collect::<Vec<_>>();
        let supplied_world_ids = canonical_world_id_set(&supplied_world_ids)?;
        if supplied_world_ids != expected_world_ids {
            return Err("supplied sealed world snapshots must match exactly the registered group".into());
        }
        for world in holdout_worlds {
            world.kernel.validate()?;
        }
        validate_opaque_id(
            &evaluation_artifact_id,
            MAX_RECEIPT_ID_LENGTH,
            "holdout evaluation artifact ID",
        )?;
        if self.events.len() >= MAX_LEDGER_EVENTS {
            return Err("model-family ledger event cap would be exceeded".into());
        }

        let holdout_nominal_world_id = expected_world_ids
            .first()
            .ok_or_else(|| "registered holdout group has no nominal reference world".to_string())?;
        let replay_result = replay_shared_policy_across_worlds(
            holdout_worlds,
            holdout_nominal_world_id,
            &self.replay_context.loss_table,
            self.replay_context.protocol,
            &self.replay_context.command_costs,
            self.replay_context.hazard_loss,
            frozen_plan,
            self.replay_context.horizon,
        );

        // Once the group and policy bindings are valid, the attempt is
        // consumed regardless of replay success. This prevents retrying the
        // same sealed group with altered world snapshots after observing an
        // unsupported branch or arithmetic failure.
        let (replay_succeeded, replay_error, replayed_ids, per_world_metrics,
            worst_case_total_loss, worst_case_terminal_loss, acceptance_criteria_satisfied) =
            match replay_result {
                Ok(replay) => {
                    let replayed_ids = replay
                        .per_world
                        .iter()
                        .map(|metrics| metrics.world_id.clone())
                        .collect::<Vec<_>>();
                    let replayed_ids = canonical_world_id_set(&replayed_ids)?;
                    if replayed_ids != expected_world_ids {
                        (false, Some("sealed policy replay did not cover the registered holdout group".into()),
                            expected_world_ids.clone(), Vec::new(), None, None, false)
                    } else {
                        let worst_case_total_loss = replay
                            .per_world
                            .iter()
                            .map(WorldDecisionObjectives::expected_total_loss)
                            .collect::<Result<Vec<_>, _>>()?
                            .into_iter()
                            .max()
                            .unwrap_or_else(Rational::zero);
                        let worst_case_terminal_loss = replay
                            .per_world
                            .iter()
                            .map(|metrics| metrics.expected_terminal_loss)
                            .max()
                            .unwrap_or_else(Rational::zero);
                        let per_world_budgets_pass = replay.per_world.iter().all(|metrics| {
                            metrics.hazard_probability <= criteria.budgets.max_hazard_probability
                                && metrics.abstention_probability <= criteria.budgets.max_abstention_probability
                        });
                        let acceptance = per_world_budgets_pass
                            && worst_case_total_loss <= criteria.max_worst_case_total_loss
                            && worst_case_terminal_loss <= criteria.max_worst_case_terminal_loss;
                        (true, None, replayed_ids, replay.per_world,
                            Some(worst_case_total_loss), Some(worst_case_terminal_loss), acceptance)
                    }
                }
                Err(error) => (
                    false,
                    Some(error),
                    expected_world_ids.clone(),
                    Vec::new(),
                    None,
                    None,
                    false,
                ),
            };

        let record = SealedHoldoutEvaluationRecord {
            event_receipt_id: event_receipt_id.clone(),
            family_group_id: family_group_id.clone(),
            manifest_id: registration.manifest_id.clone(),
            family_revision,
            policy_identity,
            world_ids: replayed_ids,
            holdout_world_snapshots: holdout_worlds.to_vec(),
            evaluation_artifact_id,
            criteria,
            replay_succeeded,
            replay_error,
            per_world_metrics,
            worst_case_total_loss,
            worst_case_terminal_loss,
            acceptance_criteria_satisfied,
        };
        self.consumed_holdout_group_ids.insert(family_group_id);
        self.append_event(
            event_receipt_id,
            ModelFamilyLedgerEvent::SealedHoldoutEvaluated(record.clone()),
        )?;
        Ok(record)
    }

    pub fn all_sealed_holdouts_consumed(&self) -> bool {
        self.reserved_holdouts
            .keys()
            .all(|group| self.consumed_holdout_group_ids.contains(group))
    }

    fn ensure_refinement_open(&self) -> Result<(), String> {
        if self.frozen_policy_identity.is_some() {
            return Err("model-family refinement is forbidden after final policy sealing".into());
        }
        if !self.consumed_holdout_group_ids.is_empty() {
            return Err("model-family refinement is forbidden after sealed evaluation".into());
        }
        Ok(())
    }

    fn validate_new_receipt_id(&self, receipt_id: &str) -> Result<(), String> {
        validate_opaque_id(receipt_id, MAX_RECEIPT_ID_LENGTH, "ledger event receipt ID")?;
        if self.event_receipt_ids.contains(receipt_id) {
            return Err("ledger event receipt IDs must be unique".into());
        }
        Ok(())
    }

    fn append_event(
        &mut self,
        receipt_id: String,
        event: ModelFamilyLedgerEvent,
    ) -> Result<(), String> {
        if self.event_receipt_ids.contains(&receipt_id) {
            return Err("ledger event receipt IDs must be unique".into());
        }
        let sequence = u64::try_from(self.events.len())
            .map_err(|_| "ledger event sequence overflow".to_string())?;
        self.event_receipt_ids.insert(receipt_id);
        self.events.push(SequencedLedgerEvent { sequence, event });
        Ok(())
    }
}

fn counterexample_outcomes_match(
    claimed: &CounterexampleOutcome,
    observed: &CounterexampleOutcome,
) -> bool {
    match (claimed, observed) {
        // The independently replayed parser/classifier may produce a more
        // specific error string than the search harness. The typed class must
        // still agree, and the observed error is retained in the admission.
        (
            CounterexampleOutcome::PolicyInvalidated { .. },
            CounterexampleOutcome::PolicyInvalidated { .. },
        ) => true,
        _ => claimed == observed,
    }
}

fn validate_opaque_id(value: &str, max_length: usize, label: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > max_length {
        return Err(format!("{label} must be non-empty and no longer than {max_length} bytes"));
    }
    Ok(())
}

fn validate_world_identity(world: &DecisionModelWorld) -> Result<(), String> {
    validate_opaque_id(&world.id, MAX_WORLD_ID_LENGTH, "candidate world ID")?;
    world.kernel.validate()
}

fn validate_validation_receipt(
    receipt: &CounterexampleValidationReceipt,
) -> Result<(), String> {
    validate_opaque_id(
        &receipt.receipt_id,
        MAX_RECEIPT_ID_LENGTH,
        "validation receipt ID",
    )?;
    validate_opaque_id(
        &receipt.validator_id,
        MAX_RECEIPT_ID_LENGTH,
        "validator ID",
    )?;
    validate_opaque_id(
        &receipt.validator_version,
        MAX_RECEIPT_ID_LENGTH,
        "validator version",
    )?;
    validate_opaque_id(
        &receipt.evidence_artifact_id,
        MAX_RECEIPT_ID_LENGTH,
        "validation evidence artifact ID",
    )?;
    if !receipt.schema_valid
        || !receipt.physical_plausibility_passed
        || !receipt.independent_provenance_passed
        || !receipt.origin_split_verified
    {
        return Err("counterexample validation receipt contains a failed admission check".into());
    }
    Ok(())
}

fn validate_provenance(provenance: &CounterexampleProvenance) -> Result<(), String> {
    validate_opaque_id(
        &provenance.family_group_id,
        MAX_WORLD_ID_LENGTH,
        "candidate family-group ID",
    )?;
    validate_opaque_id(
        &provenance.generator_id,
        MAX_RECEIPT_ID_LENGTH,
        "counterexample generator ID",
    )?;
    validate_opaque_id(
        &provenance.generator_version,
        MAX_RECEIPT_ID_LENGTH,
        "counterexample generator version",
    )?;
    validate_opaque_id(
        &provenance.search_run_id,
        MAX_RECEIPT_ID_LENGTH,
        "counterexample search-run ID",
    )?;
    validate_opaque_id(
        &provenance.policy_identity,
        MAX_RECEIPT_ID_LENGTH,
        "attacked policy identity",
    )?;
    Ok(())
}

fn canonical_world_id_set(ids: &[String]) -> Result<Vec<String>, String> {
    let mut canonical = ids.to_vec();
    for id in &canonical {
        validate_opaque_id(id, MAX_WORLD_ID_LENGTH, "world ID in replay receipt")?;
    }
    canonical.sort();
    if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("world IDs in a replay receipt must be unique".into());
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionRealizationKernel, BeliefMass, KernelRow, LatentState, ObservationField,
        PublicObservation, Rational, TransitionOutcome,
    };
    use super::super::counterexample_search::{
        CounterexampleOutcome, CounterexampleWitness, PerturbationSpec,
    };

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn world(id: &str) -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            for command in 0..2 {
                rows.push(KernelRow {
                    state,
                    command,
                    outcomes: vec![TransitionOutcome {
                        probability: Rational::one(),
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: Some(hypothesis),
                        hazard: false,
                    }],
                });
            }
        }
        DecisionModelWorld {
            id: id.into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["read-0".into(), "read-1".into()],
                realized_action_ids: vec!["executed".into(), "dropped".into()],
                readback_ids: vec!["ok".into()],
                measurement_ids: vec!["zero".into(), "one".into()],
                initial_belief: vec![
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 0,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: q(1, 2),
                    },
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 1,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: q(1, 2),
                    },
                ],
                rows,
            },
        }
    }

    fn holdout() -> SealedHoldoutGroupRegistration {
        SealedHoldoutGroupRegistration {
            family_group_id: "sealed-mechanism-family".into(),
            world_ids: vec!["heldout-world-a".into(), "heldout-world-b".into()],
            manifest_id: "heldout-manifest-v1".into(),
        }
    }

    fn holdout_worlds() -> Vec<DecisionModelWorld> {
        let first = world("heldout-world-a");
        let mut shifted = world("heldout-world-b");
        for row in &mut shifted.kernel.rows {
            for outcome in &mut row.outcomes {
                outcome.measurement = outcome.measurement.map(|measurement| 1 - measurement);
            }
        }
        vec![first, shifted]
    }

    fn holdout_criteria(max_loss: Rational) -> SealedHoldoutAcceptanceCriteria {
        SealedHoldoutAcceptanceCriteria {
            max_worst_case_total_loss: max_loss,
            max_worst_case_terminal_loss: max_loss,
            budgets: DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::zero(),
            },
        }
    }

    fn evaluate_holdout(
        ledger: &mut ModelFamilyLedger,
        event_id: &str,
        group_id: &str,
        family_revision: u64,
        policy_id: &str,
        worlds: &[DecisionModelWorld],
        criteria: SealedHoldoutAcceptanceCriteria,
    ) -> Result<SealedHoldoutEvaluationRecord, String> {
        ledger.record_sealed_holdout_evaluation(
            event_id.into(),
            group_id.into(),
            family_revision,
            policy_id.into(),
            worlds,
            criteria,
            format!("{event_id}-artifact"),
        )
    }

    fn loss_table() -> DecisionLossTable {
        DecisionLossTable {
            decision_ids: vec!["claim-h0".into(), "claim-h1".into()],
            losses: vec![
                vec![
                    vec![vec![vec![Rational::zero()]]],
                    vec![vec![vec![Rational::one()]]],
                ],
                vec![
                    vec![vec![vec![Rational::one()]]],
                    vec![vec![vec![Rational::zero()]]],
                ],
            ],
            abstention_loss: Rational::one(),
        }
    }

    fn terminal_plan() -> DecisionPlan {
        DecisionPlan::TerminalDecision { decision: Some(0) }
    }

    fn test_policy() -> DecisionPlan {
        DecisionPlan::Experiment {
            command: 0,
            branches: vec![
                super::super::finite_horizon::DecisionBranch {
                    observation: PublicObservation {
                        hazard: false,
                        readback: ObservationField::Value(0),
                        measurement: ObservationField::Value(0),
                    },
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
                super::super::finite_horizon::DecisionBranch {
                    observation: PublicObservation {
                        hazard: false,
                        readback: ObservationField::Value(0),
                        measurement: ObservationField::Value(1),
                    },
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
            ],
        }
    }

    fn recovered_policy() -> DecisionPlan {
        DecisionPlan::Experiment {
            command: 0,
            branches: vec![
                super::super::finite_horizon::DecisionBranch {
                    observation: PublicObservation {
                        hazard: false,
                        readback: ObservationField::Value(0),
                        measurement: ObservationField::Value(0),
                    },
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
                super::super::finite_horizon::DecisionBranch {
                    observation: PublicObservation {
                        hazard: false,
                        readback: ObservationField::Value(0),
                        measurement: ObservationField::Value(1),
                    },
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
                super::super::finite_horizon::DecisionBranch {
                    observation: PublicObservation {
                        hazard: false,
                        readback: ObservationField::Missing,
                        measurement: ObservationField::Value(0),
                    },
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
                super::super::finite_horizon::DecisionBranch {
                    observation: PublicObservation {
                        hazard: false,
                        readback: ObservationField::Missing,
                        measurement: ObservationField::Value(1),
                    },
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
            ],
        }
    }

    fn ledger() -> ModelFamilyLedger {
        ModelFamilyLedger::new(
            vec![world("nominal")],
            vec![holdout()],
            "policy-v1".into(),
            test_policy(),
            "nominal",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &[Rational::zero(), Rational::zero()],
            Rational::one(),
            2,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::zero(),
            },
        )
        .unwrap()
    }

    fn replan(
        ledger: &mut ModelFamilyLedger,
        event_id: &str,
        revision: u64,
        policy_id: &str,
        plan: DecisionPlan,
    ) -> Result<FamilyReplanRecord, String> {
        ledger.record_replan(event_id.into(), revision, policy_id.into(), plan)
    }

    fn counterexample(source: &DecisionModelWorld, candidate_id: &str) -> CounterexampleWitness {
        let perturbation = ModelWorldPerturbation::DropReadback {
            command: 0,
            sensor_state: Some(0),
            dropout_probability: q(1, 2),
        };
        let candidate = super::super::counterexample_search::apply_model_world_perturbation(
            source,
            candidate_id,
            &perturbation,
        )
        .unwrap();
        CounterexampleWitness {
            spec: PerturbationSpec {
                source_world_id: source.id.clone(),
                candidate_world_id: candidate_id.into(),
                perturbation,
            },
            candidate_world: candidate,
            outcome: CounterexampleOutcome::PolicyInvalidated {
                reason: "search harness reports a newly reachable missing-readback branch".into(),
            },
        }
    }

    fn provenance(
        policy_id: &str,
        split: EvidenceSplit,
        group: &str,
    ) -> CounterexampleProvenance {
        CounterexampleProvenance {
            split,
            family_group_id: group.into(),
            generator_id: "bounded-grid".into(),
            generator_version: "v1".into(),
            search_run_id: "search-run-01".into(),
            seed: 42,
            family_revision: if policy_id == "policy-v2" { 1 } else { 0 },
            policy_identity: policy_id.into(),
            attacked_policy: match policy_id {
                "policy-v1" => test_policy(),
                "policy-v2" => recovered_policy(),
                _ => terminal_plan(),
            },
        }
    }

    fn validation() -> CounterexampleValidationReceipt {
        CounterexampleValidationReceipt {
            receipt_id: "validation-01".into(),
            validator_id: "independent-physical-check".into(),
            validator_version: "v1".into(),
            evidence_artifact_id: "validation-evidence-01".into(),
            schema_valid: true,
            physical_plausibility_passed: true,
            independent_provenance_passed: true,
            origin_split_verified: true,
        }
    }

    #[test]
    fn invalid_initial_policy_cannot_enter_the_ledger_as_active() {
        let result = ModelFamilyLedger::new(
            vec![world("nominal")],
            vec![holdout()],
            "policy-invalid".into(),
            DecisionPlan::Experiment {
                command: 0,
                branches: Vec::new(),
            },
            "nominal",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &[Rational::zero(), Rational::zero()],
            Rational::one(),
            2,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::zero(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn admission_appends_candidate_increments_revision_and_invalidates_policy() {
        let mut ledger = ledger();
        let source = world("nominal");
        let witness = counterexample(&source, "sensor-inversion");
        let record = ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-family-a"),
                validation(),
                "event-admit-01".into(),
            )
            .unwrap();

        assert_eq!(record.revision_before, 0);
        assert_eq!(record.revision_after, 1);
        assert_eq!(record.invalidated_policy_identity, "policy-v1");
        assert_eq!(
            record.independently_replayed_outcome.class(),
            CounterexampleClass::PolicyInvalidated
        );
        assert_eq!(ledger.family_revision(), 1);
        assert_eq!(ledger.active_policy_identity(), None);
        assert_eq!(
            ledger.development_world_ids(),
            vec!["nominal".to_string(), "sensor-inversion".to_string()]
        );
        assert_eq!(ledger.events().len(), 1);
        assert_eq!(ledger.events()[0].sequence, 0);
    }

    #[test]
    fn policy_cannot_reactivate_until_exact_current_family_replay() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "sensor-inversion");
        ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-family-a"),
                validation(),
                "event-admit".into(),
            )
            .unwrap();

        // The invalidated policy identity cannot be reused for a new
        // policy tree, even if the caller supplies a different plan.
        assert!(replan(
            &mut ledger,
            "replan-reuses-invalidated-id",
            1,
            "policy-v1",
            terminal_plan(),
        )
        .is_err());

        // A malformed policy with missing public-observation branches is
        // rejected by the ledger's own exact shared-policy replay.
        assert!(replan(
            &mut ledger,
            "replan-incomplete-policy",
            1,
            "policy-v2",
            DecisionPlan::Experiment {
                command: 0,
                branches: Vec::new(),
            },
        )
        .is_err());
        assert!(replan(
            &mut ledger,
            "replan-stale",
            0,
            "policy-v2",
            terminal_plan(),
        )
        .is_err());
        assert_eq!(ledger.active_policy_identity(), None);

        let receipt = replan(
            &mut ledger,
            "replan-complete",
            1,
            "policy-v2",
            terminal_plan(),
        )
        .unwrap();
        assert_eq!(receipt.replayed_world_ids, vec!["nominal", "sensor-inversion"]);
        assert_eq!(ledger.active_policy_identity(), Some("policy-v2"));
    }

    #[test]
    fn no_violation_calibration_and_holdout_candidates_cannot_expand_family() {
        let mut ledger = ledger();
        let mut no_violation = counterexample(&world("nominal"), "no-violation");
        no_violation.outcome = CounterexampleOutcome::NoViolation {
            metrics: super::super::robust_finite_horizon::WorldDecisionObjectives {
                world_id: "no-violation".into(),
                expected_experiment_cost: Rational::zero(),
                expected_terminal_loss: Rational::zero(),
                hazard_probability: Rational::zero(),
                abstention_probability: Rational::zero(),
            },
        };
        assert!(ledger
            .admit_counterexample(
                &no_violation,
                provenance("policy-v1", EvidenceSplit::Development, "dev-family-a"),
                validation(),
                "no-violation-event".into(),
            )
            .is_err());

        for (split, group, receipt) in [
            (EvidenceSplit::Calibration, "calibration-a", "calibration-event"),
            (
                EvidenceSplit::SealedHoldout,
                "sealed-other",
                "holdout-event",
            ),
        ] {
            let witness = counterexample(&world("nominal"), receipt);
            assert!(ledger
                .admit_counterexample(
                    &witness,
                    provenance("policy-v1", split, group),
                    validation(),
                    receipt.into(),
                )
                .is_err());
        }
        assert_eq!(ledger.family_revision(), 0);
        assert_eq!(ledger.development_world_ids(), vec!["nominal"]);
    }

    #[test]
    fn candidate_world_id_cannot_alias_a_reserved_sealed_world() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "heldout-world-a");
        assert!(ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-not-sealed"),
                validation(),
                "candidate-aliases-holdout".into(),
            )
            .is_err());
        assert_eq!(ledger.family_revision(), 0);
        assert_eq!(ledger.development_world_ids(), vec!["nominal"]);
    }

    #[test]
    fn reserved_holdout_family_group_cannot_enter_development_search() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "leaked-holdout");
        assert!(ledger
            .admit_counterexample(
                &witness,
                provenance(
                    "policy-v1",
                    EvidenceSplit::Development,
                    "sealed-mechanism-family",
                ),
                validation(),
                "leaked-holdout-event".into(),
            )
            .is_err());
    }

    #[test]
    fn provenance_must_bind_the_current_family_revision_even_when_policy_identity_persists() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "stale-revision-candidate");
        let mut stale = provenance("policy-v1", EvidenceSplit::Development, "dev-a");
        stale.family_revision = 1;
        assert!(ledger
            .admit_counterexample(
                &witness,
                stale,
                validation(),
                "stale-family-revision".into(),
            )
            .is_err());
        assert_eq!(ledger.family_revision(), 0);
        assert_eq!(ledger.active_policy_identity(), Some("policy-v1"));
    }

    #[test]
    fn provenance_must_include_the_exact_policy_tree_not_only_its_opaque_id() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "wrong-policy-tree");
        let mut mismatched = provenance("policy-v1", EvidenceSplit::Development, "dev-a");
        mismatched.attacked_policy = terminal_plan();
        assert!(ledger
            .admit_counterexample(
                &witness,
                mismatched,
                validation(),
                "wrong-policy-tree-event".into(),
            )
            .is_err());
        assert_eq!(ledger.family_revision(), 0);
        assert_eq!(ledger.active_policy_identity(), Some("policy-v1"));
    }

    #[test]
    fn stale_source_duplicate_id_duplicate_content_and_stale_policy_reject() {
        let mut ledger = ledger();
        let valid = counterexample(&world("nominal"), "candidate-a");
        let stale_source = counterexample(&world("unknown-source"), "candidate-b");
        assert!(ledger
            .admit_counterexample(
                &stale_source,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                validation(),
                "stale-source".into(),
            )
            .is_err());
        assert!(ledger
            .admit_counterexample(
                &valid,
                provenance("old-policy", EvidenceSplit::Development, "dev-a"),
                validation(),
                "stale-policy".into(),
            )
            .is_err());
        ledger
            .admit_counterexample(
                &valid,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                validation(),
                "candidate-a-event".into(),
            )
            .unwrap();

        // The same independent-validation receipt cannot be reused for a
        // second admission, even when the event ID is new.
        assert!(ledger
            .admit_counterexample(
                &valid,
                provenance("policy-v1", EvidenceSplit::Development, "dev-b"),
                validation(),
                "duplicate-validation-event".into(),
            )
            .is_err());

        // After a complete family replan, a distinct receipt still cannot
        // re-admit the already present world ID.
        replan(
            &mut ledger,
            "replan-after-duplicate-check",
            1,
            "policy-v2",
            recovered_policy(),
        )
        .unwrap();
        assert!(ledger
            .admit_counterexample(
                &valid,
                provenance("policy-v2", EvidenceSplit::Development, "dev-b"),
                CounterexampleValidationReceipt {
                    receipt_id: "validation-02".into(),
                    evidence_artifact_id: "validation-evidence-02".into(),
                    ..validation()
                },
                "duplicate-id-event".into(),
            )
            .is_err());
    }

    #[test]
    fn candidate_world_must_be_reproduced_from_the_declared_perturbation() {
        let mut ledger = ledger();
        let mut forged = counterexample(&world("nominal"), "forged-candidate");
        for row in &mut forged.candidate_world.kernel.rows {
            for outcome in &mut row.outcomes {
                outcome.measurement = outcome.measurement.map(|measurement| 1 - measurement);
            }
        }
        // The changed candidate remains a structurally valid kernel and still
        // exposes the missing-readback branch, but it is not the exact output
        // of the declared DropReadback perturbation.
        assert!(forged.candidate_world.kernel.validate().is_ok());
        assert!(ledger
            .admit_counterexample(
                &forged,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                validation(),
                "forged-candidate-event".into(),
            )
            .is_err());
        assert_eq!(ledger.family_revision(), 0);
        assert_eq!(ledger.active_policy_identity(), Some("policy-v1"));
    }

    #[test]
    fn failed_validation_and_duplicate_candidate_content_reject() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "candidate-a");
        let mut failed = validation();
        failed.physical_plausibility_passed = false;
        assert!(ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                failed,
                "failed-validation".into(),
            )
            .is_err());

        ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                validation(),
                "candidate-a".into(),
            )
            .unwrap();

        let mut duplicate_content = counterexample(&world("nominal"), "candidate-b");
        duplicate_content.candidate_world.kernel =
            witness.candidate_world.kernel.clone();
        replan(&mut ledger, "replan", 1, "policy-v2", recovered_policy()).unwrap();
        assert!(ledger
            .admit_counterexample(
                &duplicate_content,
                provenance("policy-v2", EvidenceSplit::Development, "dev-b"),
                CounterexampleValidationReceipt {
                    receipt_id: "validation-02".into(),
                    ..validation()
                },
                "duplicate-content".into(),
            )
            .is_err());
    }

    #[test]
    fn final_policy_seal_and_holdout_evaluation_are_one_way() {
        let mut ledger = ledger();
        ledger
            .seal_final_policy("seal-policy".into(), 0, "policy-v1".into())
            .unwrap();
        assert!(ledger.is_policy_frozen());

        let worlds = holdout_worlds();
        let evaluated = evaluate_holdout(
            &mut ledger,
            "holdout-eval",
            "sealed-mechanism-family",
            0,
            "policy-v1",
            &worlds,
            holdout_criteria(Rational::zero()),
        )
        .unwrap();
        assert!(!evaluated.acceptance_criteria_satisfied);
        assert_eq!(evaluated.worst_case_terminal_loss, Some(Rational::one()));
        assert!(evaluated.replay_succeeded);
        assert_eq!(evaluated.replay_error, None);
        assert_eq!(evaluated.per_world_metrics.len(), 2);
        assert_eq!(evaluated.holdout_world_snapshots.len(), 2);
        assert_eq!(
            evaluated.manifest_id,
            "heldout-manifest-v1"
        );
        assert!(ledger.all_sealed_holdouts_consumed());
        assert!(evaluate_holdout(
            &mut ledger,
            "holdout-eval-again",
            "sealed-mechanism-family",
            0,
            "policy-v1",
            &worlds,
            holdout_criteria(Rational::one()),
        )
        .is_err());

        let witness = counterexample(&world("nominal"), "too-late");
        assert!(ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                validation(),
                "too-late-event".into(),
            )
            .is_err());
        assert!(replan(&mut ledger, "too-late-replan", 0, "policy-v2", terminal_plan()).is_err());
    }

    #[test]
    fn a_passing_sealed_holdout_result_is_computed_from_internal_replay() {
        let mut ledger = ledger();
        ledger
            .seal_final_policy("seal-policy".into(), 0, "policy-v1".into())
            .unwrap();
        let worlds = vec![world("heldout-world-a"), world("heldout-world-b")];
        let evaluated = evaluate_holdout(
            &mut ledger,
            "passing-holdout-eval",
            "sealed-mechanism-family",
            0,
            "policy-v1",
            &worlds,
            holdout_criteria(Rational::zero()),
        )
        .unwrap();
        assert!(evaluated.acceptance_criteria_satisfied);
        assert_eq!(evaluated.worst_case_terminal_loss, Some(Rational::zero()));
        assert!(evaluated.replay_succeeded);
        assert!(ledger.all_sealed_holdouts_consumed());
    }

    #[test]
    fn failed_sealed_policy_replay_consumes_the_holdout_once() {
        let mut ledger = ledger();
        ledger
            .seal_final_policy("seal-policy".into(), 0, "policy-v1".into())
            .unwrap();
        let invalid_source = world("heldout-source");
        let perturbation = ModelWorldPerturbation::DropReadback {
            command: 0,
            sensor_state: Some(0),
            dropout_probability: q(1, 2),
        };
        let invalid_world =
            super::super::counterexample_search::apply_model_world_perturbation(
                &invalid_source,
                "heldout-world-b",
                &perturbation,
            )
            .unwrap();
        let worlds = vec![world("heldout-world-a"), invalid_world];

        let result = evaluate_holdout(
            &mut ledger,
            "failed-replay-holdout",
            "sealed-mechanism-family",
            0,
            "policy-v1",
            &worlds,
            holdout_criteria(Rational::one()),
        )
        .unwrap();
        assert!(!result.replay_succeeded);
        assert!(result.replay_error.is_some());
        assert!(!result.acceptance_criteria_satisfied);
        assert!(ledger.all_sealed_holdouts_consumed());

        let retry = evaluate_holdout(
            &mut ledger,
            "failed-replay-holdout-retry",
            "sealed-mechanism-family",
            0,
            "policy-v1",
            &holdout_worlds(),
            holdout_criteria(Rational::one()),
        );
        assert!(retry.is_err());
    }

    #[test]
    fn holdout_evaluation_must_match_registered_group_revision_policy_and_worlds() {
        let mut ledger = ledger();
        ledger
            .seal_final_policy("seal".into(), 0, "policy-v1".into())
            .unwrap();
        let worlds = holdout_worlds();
        let missing_world = vec![world("heldout-world-a")];

        assert!(evaluate_holdout(
            &mut ledger,
            "wrong-worlds",
            "sealed-mechanism-family",
            0,
            "policy-v1",
            &missing_world,
            holdout_criteria(Rational::one()),
        )
        .is_err());
        assert!(evaluate_holdout(
            &mut ledger,
            "wrong-policy",
            "sealed-mechanism-family",
            0,
            "policy-other",
            &worlds,
            holdout_criteria(Rational::one()),
        )
        .is_err());
        assert!(evaluate_holdout(
            &mut ledger,
            "wrong-revision",
            "sealed-mechanism-family",
            1,
            "policy-v1",
            &worlds,
            holdout_criteria(Rational::one()),
        )
        .is_err());
        assert!(evaluate_holdout(
            &mut ledger,
            "unknown-group",
            "not-registered",
            0,
            "policy-v1",
            &worlds,
            holdout_criteria(Rational::one()),
        )
        .is_err());
    }

    #[test]
    fn duplicate_receipt_ids_and_invalid_registrations_fail_closed() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "candidate");
        ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                validation(),
                "same-event".into(),
            )
            .unwrap();
        assert!(replan(&mut ledger, "same-event", 1, "policy-v2", terminal_plan()).is_err());

        assert!(ModelFamilyLedger::new(
            vec![world("nominal"), world("nominal")],
            vec![],
            "policy".into(),
            terminal_plan(),
            "nominal",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &[Rational::zero(), Rational::zero()],
            Rational::one(),
            2,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::zero(),
            },
        )
        .is_err());
        let duplicate_holdout = SealedHoldoutGroupRegistration {
            family_group_id: "duplicate".into(),
            world_ids: vec!["heldout".into(), "heldout".into()],
            manifest_id: "manifest".into(),
        };
        assert!(ModelFamilyLedger::new(
            vec![world("nominal")],
            vec![duplicate_holdout],
            "policy".into(),
            terminal_plan(),
            "nominal",
            &loss_table(),
            ObservationProtocol::fully_observed(),
            &[Rational::zero(), Rational::zero()],
            Rational::one(),
            2,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::zero(),
            },
        )
        .is_err());
    }

    #[test]
    fn event_revisions_and_snapshots_are_append_only_and_canonical() {
        let mut ledger = ledger();
        let witness = counterexample(&world("nominal"), "candidate");
        ledger
            .admit_counterexample(
                &witness,
                provenance("policy-v1", EvidenceSplit::Development, "dev-a"),
                validation(),
                "admit".into(),
            )
            .unwrap();
        replan(&mut ledger, "replan", 1, "policy-v2", terminal_plan()).unwrap();
        let snapshot = ledger.snapshot();
        assert_eq!(snapshot.family_revision, 1);
        assert!(snapshot.active_policy_metrics.is_some());
        assert!(snapshot.active_policy_plan.is_some());
        assert_eq!(snapshot.replay_context.nominal_world_id, "nominal");
        assert_eq!(snapshot.events.len(), 2);
        assert_eq!(
            snapshot.events.iter().map(|event| event.sequence).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!(snapshot.events.windows(2).all(|pair| {
            pair[0].event.family_revision() <= pair[1].event.family_revision()
        }));
    }
}
