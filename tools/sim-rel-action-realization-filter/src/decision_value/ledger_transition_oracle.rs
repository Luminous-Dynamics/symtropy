//! Independent logical transition oracle for SIM-REL-034 model-family snapshots.
//!
//! This module intentionally does not call ModelFamilyLedger, its transition
//! methods, the physical perturbation helper, or the policy replay engine.
//! It independently derives the ledger's logical lifecycle from the immutable
//! baseline and sequenced records, then compares that derived state with the
//! exported snapshot.
//!
//! Independence boundary:
//! - Checks: event order, revision arithmetic, policy invalidation/reactivation,
//!   split isolation, identity uniqueness, sealing, one-time holdout consumption,
//!   metric-record structure, and final snapshot consistency.
//! - Does not check: whether a perturbation correctly changes physical dynamics,
//!   whether a policy actually produces its recorded metrics, whether an
//!   external split registry is truthful, or whether the declared model family
//!   covers the real world. Those require separate domain and provenance checks.

use super::counterexample_ledger::{
    CalibrationGroupRegistration, CounterexampleAdmissionRecord, CounterexampleValidationReceipt,
    EvidenceSplit, FinalPolicySealRecord, ModelFamilyLedgerEvent, ModelFamilyLedgerSnapshot,
    SealedHoldoutEvaluationRecord, SealedHoldoutGroupRegistration, SequencedLedgerEvent,
};
use super::counterexample_search::CounterexampleClass;
use super::finite_horizon::{DecisionBudgets, DecisionPlan};
use super::robust_finite_horizon::{DecisionModelWorld, WorldDecisionObjectives};
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_EVENTS: usize = 100_000;
const MAX_DEVELOPMENT_WORLDS: usize = 16;
const MAX_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionOraclePhase {
    SnapshotEnvelope,
    EventSequence,
    CounterexampleAdmission,
    PolicyReplan,
    FinalPolicySeal,
    SealedHoldoutEvaluation,
    FinalSnapshotEquality,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionOracleError {
    pub event_sequence: Option<u64>,
    pub phase: TransitionOraclePhase,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentTransitionAuditReceipt {
    pub event_count: u64,
    pub counterexample_admission_count: u64,
    pub policy_replan_count: u64,
    pub final_policy_seal_count: u64,
    pub sealed_holdout_evaluation_count: u64,
    pub final_family_revision: u64,
    pub development_world_ids: Vec<String>,
    pub consumed_holdout_group_ids: Vec<String>,
    pub active_policy_identity: Option<String>,
    pub frozen_policy_identity: Option<String>,
    pub logical_snapshot_exactly_reconstructed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PolicyState {
    identity: String,
    plan: DecisionPlan,
    // Initial baseline metrics are opaque to this logical oracle. Replan and
    // seal records make subsequent metric values available for exact equality.
    metrics: Option<Vec<WorldDecisionObjectives>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FrozenPolicyState {
    identity: String,
    plan: DecisionPlan,
    metrics: Vec<WorldDecisionObjectives>,
    revision: u64,
}

#[derive(Clone, Debug)]
struct TransitionState {
    revision: u64,
    initial_policy_metrics: Vec<WorldDecisionObjectives>,
    development_worlds: Vec<DecisionModelWorld>,
    development_world_ids: BTreeSet<String>,
    active_policy: Option<PolicyState>,
    frozen_policy: Option<FrozenPolicyState>,
    invalidated_policy_identity: Option<String>,
    policy_identities: BTreeSet<String>,
    validation_receipt_ids: BTreeSet<String>,
    event_receipt_ids: BTreeSet<String>,
    evaluation_artifact_ids: BTreeSet<String>,
    consumed_holdout_group_ids: BTreeSet<String>,
    calibration_groups: BTreeMap<String, CalibrationGroupRegistration>,
    holdout_groups: BTreeMap<String, SealedHoldoutGroupRegistration>,
    counts: [u64; 4],
}

fn fail(
    event_sequence: Option<u64>,
    phase: TransitionOraclePhase,
    reason: impl Into<String>,
) -> TransitionOracleError {
    TransitionOracleError {
        event_sequence,
        phase,
        reason: reason.into(),
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn sorted_unique_ids(values: &[String], label: &str) -> Result<Vec<String>, String> {
    let mut result = values.to_vec();
    if result.iter().any(|value| !valid_id(value)) {
        return Err(format!("{label} contains an empty or overlong identifier"));
    }
    result.sort();
    if result.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(format!("{label} contains duplicate identifiers"));
    }
    Ok(result)
}

fn sorted_metric_ids(
    metrics: &[WorldDecisionObjectives],
    label: &str,
) -> Result<Vec<String>, String> {
    let ids = metrics
        .iter()
        .map(|metric| metric.world_id.clone())
        .collect::<Vec<_>>();
    sorted_unique_ids(&ids, label)
}

fn validate_metric_values(
    metrics: &[WorldDecisionObjectives],
    expected_world_ids: &[String],
    budgets: DecisionBudgets,
    label: &str,
) -> Result<(), String> {
    let actual_ids = sorted_metric_ids(metrics, label)?;
    if actual_ids != expected_world_ids {
        return Err(format!(
            "{label} does not cover exactly the expected world set"
        ));
    }
    if !budgets.max_hazard_probability.is_probability()
        || !budgets.max_abstention_probability.is_probability()
    {
        return Err(format!("{label} uses an invalid probability budget"));
    }
    for metric in metrics {
        if !metric.hazard_probability.is_probability()
            || !metric.abstention_probability.is_probability()
        {
            return Err(format!(
                "{label} contains an invalid risk probability for world {}",
                metric.world_id
            ));
        }
        metric
            .expected_total_loss()
            .map_err(|reason| format!("{label} total-loss arithmetic failed: {reason}"))?;
        if metric.hazard_probability > budgets.max_hazard_probability {
            return Err(format!(
                "{label} exceeds the hazard budget in world {}",
                metric.world_id
            ));
        }
        if metric.abstention_probability > budgets.max_abstention_probability {
            return Err(format!(
                "{label} exceeds the abstention budget in world {}",
                metric.world_id
            ));
        }
    }
    Ok(())
}

fn validate_validation_receipt(receipt: &CounterexampleValidationReceipt) -> Result<(), String> {
    for (value, label) in [
        (&receipt.receipt_id, "validation receipt ID"),
        (&receipt.validator_id, "validator ID"),
        (&receipt.validator_version, "validator version"),
        (
            &receipt.evidence_artifact_id,
            "validation evidence artifact ID",
        ),
    ] {
        if !valid_id(value) {
            return Err(format!("{label} is empty or overlong"));
        }
    }
    if !receipt.schema_valid
        || !receipt.physical_plausibility_passed
        || !receipt.independent_provenance_passed
        || !receipt.origin_split_verified
    {
        return Err("counterexample admission lacks a complete passing validation receipt".into());
    }
    Ok(())
}

impl TransitionState {
    fn from_snapshot(snapshot: &ModelFamilyLedgerSnapshot) -> Result<Self, String> {
        if snapshot.events.len() > MAX_EVENTS {
            return Err("snapshot event count exceeds the independent oracle cap".into());
        }
        if snapshot.baseline_development_worlds.is_empty()
            || snapshot.baseline_development_worlds.len() > MAX_DEVELOPMENT_WORLDS
        {
            return Err("baseline development family has an unsupported size".into());
        }
        if !valid_id(&snapshot.initial_policy_identity) {
            return Err("initial policy identity is empty or overlong".into());
        }
        if !valid_id(&snapshot.replay_context.nominal_world_id) {
            return Err("nominal world identity is empty or overlong".into());
        }
        if !snapshot
            .replay_context
            .development_budgets
            .max_hazard_probability
            .is_probability()
            || !snapshot
                .replay_context
                .development_budgets
                .max_abstention_probability
                .is_probability()
        {
            return Err("development risk budgets are outside [0,1]".into());
        }

        let mut development_worlds = snapshot.baseline_development_worlds.clone();
        development_worlds.sort_by(|left, right| left.id.cmp(&right.id));
        if development_worlds != snapshot.baseline_development_worlds {
            return Err("baseline development worlds are not in canonical ID order".into());
        }
        let baseline_ids = development_worlds
            .iter()
            .map(|world| world.id.clone())
            .collect::<Vec<_>>();
        let baseline_ids = sorted_unique_ids(&baseline_ids, "baseline world IDs")?;
        if !baseline_ids
            .iter()
            .any(|world_id| world_id == &snapshot.replay_context.nominal_world_id)
        {
            return Err("nominal world is absent from the baseline development family".into());
        }

        let mut all_world_ids = baseline_ids.iter().cloned().collect::<BTreeSet<_>>();
        let mut calibration_groups = BTreeMap::new();
        let mut holdout_groups = BTreeMap::new();
        let mut all_group_ids = BTreeSet::new();

        for registration in &snapshot.reserved_calibration_groups {
            if !valid_id(&registration.family_group_id)
                || !valid_id(&registration.manifest_id)
                || registration.world_ids.is_empty()
            {
                return Err(
                    "calibration registration has an invalid ID, manifest, or empty world set"
                        .into(),
                );
            }
            let world_ids = sorted_unique_ids(
                &registration.world_ids,
                "calibration registration world IDs",
            )?;
            if world_ids != registration.world_ids {
                return Err("calibration registration world IDs are not canonical".into());
            }
            if !all_group_ids.insert(registration.family_group_id.clone()) {
                return Err("family-group IDs collide across evidence splits".into());
            }
            for world_id in &world_ids {
                if !all_world_ids.insert(world_id.clone()) {
                    return Err("world IDs collide across evidence splits".into());
                }
            }
            if calibration_groups
                .insert(registration.family_group_id.clone(), registration.clone())
                .is_some()
            {
                return Err("calibration family-group ID is duplicated".into());
            }
        }

        for registration in &snapshot.reserved_holdout_groups {
            if !valid_id(&registration.family_group_id)
                || !valid_id(&registration.manifest_id)
                || registration.world_ids.is_empty()
            {
                return Err(
                    "sealed registration has an invalid ID, manifest, or empty world set".into(),
                );
            }
            let world_ids =
                sorted_unique_ids(&registration.world_ids, "sealed registration world IDs")?;
            if world_ids != registration.world_ids {
                return Err("sealed registration world IDs are not canonical".into());
            }
            if !all_group_ids.insert(registration.family_group_id.clone()) {
                return Err("family-group IDs collide across evidence splits".into());
            }
            for world_id in &world_ids {
                if !all_world_ids.insert(world_id.clone()) {
                    return Err("world IDs collide across evidence splits".into());
                }
            }
            if holdout_groups
                .insert(registration.family_group_id.clone(), registration.clone())
                .is_some()
            {
                return Err("sealed family-group ID is duplicated".into());
            }
        }

        let development_world_ids = baseline_ids.iter().cloned().collect::<BTreeSet<_>>();
        let initial_policy_metrics = snapshot.initial_policy_metrics.clone();
        validate_metric_values(
            &initial_policy_metrics,
            &baseline_ids,
            snapshot.replay_context.development_budgets,
            "initial policy metrics",
        )?;
        let policy_identities = BTreeSet::from([snapshot.initial_policy_identity.clone()]);
        Ok(Self {
            revision: 0,
            initial_policy_metrics: initial_policy_metrics.clone(),
            development_worlds,
            development_world_ids,
            active_policy: Some(PolicyState {
                identity: snapshot.initial_policy_identity.clone(),
                plan: snapshot.initial_policy_plan.clone(),
                metrics: Some(initial_policy_metrics),
            }),
            frozen_policy: None,
            invalidated_policy_identity: None,
            policy_identities,
            validation_receipt_ids: BTreeSet::new(),
            event_receipt_ids: BTreeSet::new(),
            evaluation_artifact_ids: BTreeSet::new(),
            consumed_holdout_group_ids: BTreeSet::new(),
            calibration_groups,
            holdout_groups,
            counts: [0; 4],
        })
    }

    fn register_event_id(&mut self, event_id: &str) -> Result<(), String> {
        if !valid_id(event_id) {
            return Err("event receipt ID is empty or overlong".into());
        }
        if !self.event_receipt_ids.insert(event_id.to_string()) {
            return Err("event receipt ID is reused".into());
        }
        Ok(())
    }

    fn admit(&mut self, record: &CounterexampleAdmissionRecord) -> Result<(), String> {
        if self.frozen_policy.is_some() || !self.consumed_holdout_group_ids.is_empty() {
            return Err(
                "counterexample admission is forbidden after final sealing or holdout use".into(),
            );
        }
        if record.revision_before != self.revision {
            return Err("counterexample admission starts from a stale family revision".into());
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "family revision overflow".to_string())?;
        if record.revision_after != next_revision {
            return Err(
                "counterexample admission does not increment the revision exactly once".into(),
            );
        }

        let active = self.active_policy.as_ref().ok_or_else(|| {
            "counterexample admission occurred without an active policy".to_string()
        })?;
        if record.invalidated_policy_identity != active.identity {
            return Err("counterexample invalidates a policy other than the active policy".into());
        }
        if record.provenance.policy_identity != active.identity
            || record.provenance.attacked_policy != active.plan
            || record.provenance.family_revision != self.revision
        {
            return Err(
                "counterexample provenance is not bound to the active policy and current revision"
                    .into(),
            );
        }
        if record.provenance.split != EvidenceSplit::Development {
            return Err("only Development evidence may expand the model family".into());
        }
        if !valid_id(&record.provenance.family_group_id)
            || !valid_id(&record.provenance.generator_id)
            || !valid_id(&record.provenance.generator_version)
            || !valid_id(&record.provenance.search_run_id)
        {
            return Err("counterexample provenance contains an invalid identity".into());
        }
        if self
            .calibration_groups
            .contains_key(&record.provenance.family_group_id)
            || self
                .holdout_groups
                .contains_key(&record.provenance.family_group_id)
        {
            return Err("Development counterexample aliases a reserved group".into());
        }

        let source = self
            .development_worlds
            .iter()
            .find(|world| world.id == record.source_world_id)
            .ok_or_else(|| "counterexample source world is absent".to_string())?;
        if source != &record.source_world_snapshot {
            return Err("counterexample source snapshot differs from the current family".into());
        }
        if !valid_id(&record.candidate_world_id)
            || record.candidate_world_id == record.source_world_id
            || record.candidate_world_snapshot.id != record.candidate_world_id
            || !self
                .development_world_ids
                .insert(record.candidate_world_id.clone())
        {
            return Err("counterexample candidate identity is invalid or already present".into());
        }
        if self.development_world_ids.len() > MAX_DEVELOPMENT_WORLDS {
            return Err("counterexample would exceed the development-world cap".into());
        }

        validate_validation_receipt(&record.validation)?;
        if !self
            .validation_receipt_ids
            .insert(record.validation.receipt_id.clone())
        {
            return Err("validation receipt ID is reused".into());
        }
        if record.claimed_outcome.class() == CounterexampleClass::NoViolation
            || record.independently_replayed_outcome.class() == CounterexampleClass::NoViolation
            || record.claimed_outcome.class() != record.independently_replayed_outcome.class()
        {
            return Err("counterexample witness is not a matching non-violation outcome".into());
        }

        self.development_worlds
            .push(record.candidate_world_snapshot.clone());
        self.revision = next_revision;
        self.invalidated_policy_identity = Some(active.identity.clone());
        self.active_policy = None;
        self.counts[0] = self.counts[0]
            .checked_add(1)
            .ok_or_else(|| "admission count overflow".to_string())?;
        Ok(())
    }

    fn replan(
        &mut self,
        record: &super::counterexample_ledger::FamilyReplanRecord,
        budgets: DecisionBudgets,
    ) -> Result<(), String> {
        if self.frozen_policy.is_some() || !self.consumed_holdout_group_ids.is_empty() {
            return Err("policy replan is forbidden after final sealing or holdout use".into());
        }
        if self.active_policy.is_some() || self.invalidated_policy_identity.is_none() {
            return Err("policy replan requires an invalidated policy".into());
        }
        if record.family_revision != self.revision {
            return Err("policy replan is bound to a stale family revision".into());
        }
        if !valid_id(&record.policy_identity)
            || self.policy_identities.contains(&record.policy_identity)
        {
            return Err("replanned policy identity is invalid or reused".into());
        }

        let expected_world_ids = self
            .development_world_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let recorded_world_ids =
            sorted_unique_ids(&record.replayed_world_ids, "replayed development world IDs")?;
        if recorded_world_ids != expected_world_ids {
            return Err("replan record does not name the exact current development family".into());
        }
        validate_metric_values(
            &record.per_world_metrics,
            &expected_world_ids,
            budgets,
            "replan metrics",
        )?;
        self.policy_identities
            .insert(record.policy_identity.clone());
        self.active_policy = Some(PolicyState {
            identity: record.policy_identity.clone(),
            plan: record.plan.clone(),
            metrics: Some(record.per_world_metrics.clone()),
        });
        self.invalidated_policy_identity = None;
        self.counts[1] = self.counts[1]
            .checked_add(1)
            .ok_or_else(|| "replan count overflow".to_string())?;
        Ok(())
    }
}

fn metrics_for_snapshot(
    metrics: &[WorldDecisionObjectives],
    expected_world_ids: &[String],
    budgets: DecisionBudgets,
    label: &str,
) -> Result<(), String> {
    validate_metric_values(metrics, expected_world_ids, budgets, label)
}

fn handle_seal(
    state: &mut TransitionState,
    record: &FinalPolicySealRecord,
    budgets: DecisionBudgets,
) -> Result<(), String> {
    if state.frozen_policy.is_some() {
        return Err("a second final-policy seal is not permitted".into());
    }
    if record.family_revision != state.revision {
        return Err("final-policy seal is bound to a stale family revision".into());
    }
    let active = state
        .active_policy
        .as_mut()
        .ok_or_else(|| "final-policy seal requires an active policy".to_string())?;
    if record.policy_identity != active.identity || record.plan != active.plan {
        return Err("final-policy seal does not match the active policy identity and tree".into());
    }
    let expected_world_ids = state
        .development_world_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    metrics_for_snapshot(
        &record.per_world_metrics,
        &expected_world_ids,
        budgets,
        "final-policy seal metrics",
    )?;
    if let Some(active_metrics) = &active.metrics {
        if active_metrics != &record.per_world_metrics {
            return Err("final-policy seal metrics differ from the active replan receipt".into());
        }
    }
    active.metrics = Some(record.per_world_metrics.clone());
    state.frozen_policy = Some(FrozenPolicyState {
        identity: record.policy_identity.clone(),
        plan: record.plan.clone(),
        metrics: record.per_world_metrics.clone(),
        revision: record.family_revision,
    });
    state.counts[2] = state.counts[2]
        .checked_add(1)
        .ok_or_else(|| "final-policy seal count overflow".to_string())?;
    Ok(())
}

fn holdout_metrics_match(
    record: &SealedHoldoutEvaluationRecord,
    expected_world_ids: &[String],
) -> Result<(), String> {
    if !record
        .criteria
        .budgets
        .max_hazard_probability
        .is_probability()
        || !record
            .criteria
            .budgets
            .max_abstention_probability
            .is_probability()
    {
        return Err("holdout risk budget is outside [0,1]".into());
    }
    if !record.replay_succeeded {
        if record
            .replay_error
            .as_ref()
            .is_none_or(|reason| reason.trim().is_empty())
            || !record.per_world_metrics.is_empty()
            || record.worst_case_total_loss.is_some()
            || record.worst_case_terminal_loss.is_some()
            || record.acceptance_criteria_satisfied
        {
            return Err("failed holdout replay has an inconsistent failure receipt".into());
        }
        return Ok(());
    }

    if record.replay_error.is_some() {
        return Err("successful holdout replay must not contain a replay error".into());
    }
    validate_metric_values(
        &record.per_world_metrics,
        expected_world_ids,
        DecisionBudgets {
            max_hazard_probability: Rational::one(),
            max_abstention_probability: Rational::one(),
        },
        "holdout replay metrics",
    )?;
    let mut worst_total = Rational::zero();
    let mut worst_terminal = Rational::zero();
    let mut budgets_pass = true;
    for metric in &record.per_world_metrics {
        let total = metric
            .expected_total_loss()
            .map_err(|reason| format!("holdout total-loss arithmetic failed: {reason}"))?;
        worst_total = worst_total.max(total);
        worst_terminal = worst_terminal.max(metric.expected_terminal_loss);
        budgets_pass &= metric.hazard_probability <= record.criteria.budgets.max_hazard_probability;
        budgets_pass &=
            metric.abstention_probability <= record.criteria.budgets.max_abstention_probability;
    }
    if record.worst_case_total_loss != Some(worst_total)
        || record.worst_case_terminal_loss != Some(worst_terminal)
    {
        return Err("holdout worst-case aggregates do not match the per-world metrics".into());
    }
    let expected_acceptance = budgets_pass
        && worst_total <= record.criteria.max_worst_case_total_loss
        && worst_terminal <= record.criteria.max_worst_case_terminal_loss;
    if record.acceptance_criteria_satisfied != expected_acceptance {
        return Err("holdout acceptance flag does not match its exact recorded criteria".into());
    }
    Ok(())
}

fn handle_holdout(
    state: &mut TransitionState,
    record: &SealedHoldoutEvaluationRecord,
) -> Result<(), String> {
    let frozen = state
        .frozen_policy
        .as_ref()
        .ok_or_else(|| "holdout evaluation occurred before final-policy sealing".to_string())?;
    if record.family_revision != frozen.revision || record.policy_identity != frozen.identity {
        return Err("holdout evaluation is not bound to the frozen policy and revision".into());
    }
    let registration = state
        .holdout_groups
        .get(&record.family_group_id)
        .ok_or_else(|| "holdout evaluation uses an unregistered group".to_string())?;
    if record.manifest_id != registration.manifest_id {
        return Err("holdout evaluation manifest differs from the registration".into());
    }
    if state
        .consumed_holdout_group_ids
        .contains(&record.family_group_id)
    {
        return Err("sealed holdout group is consumed more than once".into());
    }
    let expected_world_ids = registration.world_ids.clone();
    let record_world_ids = sorted_unique_ids(&record.world_ids, "holdout world IDs")?;
    if record_world_ids != expected_world_ids {
        return Err("holdout evaluation does not name exactly the registered worlds".into());
    }
    let snapshot_world_ids = record
        .holdout_world_snapshots
        .iter()
        .map(|world| world.id.clone())
        .collect::<Vec<_>>();
    if sorted_unique_ids(&snapshot_world_ids, "holdout snapshot world IDs")? != expected_world_ids {
        return Err("holdout snapshots do not cover exactly the registered worlds".into());
    }
    if !valid_id(&record.evaluation_artifact_id)
        || !state
            .evaluation_artifact_ids
            .insert(record.evaluation_artifact_id.clone())
    {
        return Err("holdout evaluation artifact ID is invalid or reused".into());
    }
    holdout_metrics_match(record, &expected_world_ids)?;
    state
        .consumed_holdout_group_ids
        .insert(record.family_group_id.clone());
    state.counts[3] = state.counts[3]
        .checked_add(1)
        .ok_or_else(|| "sealed evaluation count overflow".to_string())?;
    Ok(())
}

fn verify_event(
    state: &mut TransitionState,
    sequenced: &SequencedLedgerEvent,
    budgets: DecisionBudgets,
) -> Result<(), TransitionOracleError> {
    state
        .register_event_id(sequenced.event.receipt_id())
        .map_err(|reason| {
            fail(
                Some(sequenced.sequence),
                TransitionOraclePhase::EventSequence,
                reason,
            )
        })?;

    let result = match &sequenced.event {
        ModelFamilyLedgerEvent::CounterexampleAdmitted(record) => state.admit(record),
        ModelFamilyLedgerEvent::PolicyReplanned(record) => state.replan(record, budgets),
        ModelFamilyLedgerEvent::FinalPolicySealed(record) => handle_seal(state, record, budgets),
        ModelFamilyLedgerEvent::SealedHoldoutEvaluated(record) => handle_holdout(state, record),
    };

    let phase = match &sequenced.event {
        ModelFamilyLedgerEvent::CounterexampleAdmitted(_) => {
            TransitionOraclePhase::CounterexampleAdmission
        }
        ModelFamilyLedgerEvent::PolicyReplanned(_) => TransitionOraclePhase::PolicyReplan,
        ModelFamilyLedgerEvent::FinalPolicySealed(_) => TransitionOraclePhase::FinalPolicySeal,
        ModelFamilyLedgerEvent::SealedHoldoutEvaluated(_) => {
            TransitionOraclePhase::SealedHoldoutEvaluation
        }
    };
    result.map_err(|reason| fail(Some(sequenced.sequence), phase, reason))
}

fn compare_final_snapshot(
    snapshot: &ModelFamilyLedgerSnapshot,
    state: &TransitionState,
    budgets: DecisionBudgets,
) -> Result<(), String> {
    let expected_world_ids = state
        .development_world_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    if snapshot.initial_policy_metrics != state.initial_policy_metrics {
        return Err(
            "snapshot initial-policy metrics differ from the immutable baseline receipt".into(),
        );
    }
    if snapshot.family_revision != state.revision {
        return Err(
            "snapshot family revision differs from the independently derived revision".into(),
        );
    }
    if snapshot.development_worlds != state.development_worlds {
        return Err(
            "snapshot development worlds differ from the independently derived append order".into(),
        );
    }
    if snapshot.active_policy_identity != state.active_policy.as_ref().map(|p| p.identity.clone()) {
        return Err("snapshot active policy identity differs from the transition state".into());
    }
    if snapshot.active_policy_plan != state.active_policy.as_ref().map(|p| p.plan.clone()) {
        return Err("snapshot active policy tree differs from the transition state".into());
    }
    match state
        .active_policy
        .as_ref()
        .and_then(|policy| policy.metrics.as_ref())
    {
        Some(metrics) => {
            if snapshot.active_policy_metrics.as_ref() != Some(metrics) {
                return Err(
                    "snapshot active-policy metrics differ from the independent replan/seal record"
                        .into(),
                );
            }
        }
        None if state.active_policy.is_none() => {
            if snapshot.active_policy_metrics.is_some() {
                return Err("snapshot retains metrics for an invalidated or absent policy".into());
            }
        }
        None => {
            let metrics = snapshot.active_policy_metrics.as_ref().ok_or_else(|| {
                "initial active policy is missing its baseline replay metrics".to_string()
            })?;
            metrics_for_snapshot(
                metrics,
                &expected_world_ids,
                budgets,
                "baseline active-policy metrics",
            )?;
        }
    }

    if snapshot.frozen_policy_identity != state.frozen_policy.as_ref().map(|p| p.identity.clone())
        || snapshot.frozen_policy_plan != state.frozen_policy.as_ref().map(|p| p.plan.clone())
        || snapshot.frozen_policy_revision != state.frozen_policy.as_ref().map(|p| p.revision)
        || snapshot.frozen_policy_metrics != state.frozen_policy.as_ref().map(|p| p.metrics.clone())
    {
        return Err(
            "snapshot frozen-policy fields differ from the independently derived seal".into(),
        );
    }
    if snapshot.invalidated_policy_identity != state.invalidated_policy_identity {
        return Err(
            "snapshot invalidated-policy identity differs from the transition state".into(),
        );
    }

    let consumed = state
        .consumed_holdout_group_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    if snapshot.consumed_holdout_group_ids != consumed {
        return Err("snapshot consumed-holdout set differs from the transition state".into());
    }
    let used_policies = state.policy_identities.iter().cloned().collect::<Vec<_>>();
    if snapshot.used_policy_identities != used_policies {
        return Err("snapshot used-policy identity set differs from the transition state".into());
    }
    let used_validation = state
        .validation_receipt_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    if snapshot.used_validation_receipt_ids != used_validation {
        return Err("snapshot validation receipt set differs from the transition state".into());
    }
    let used_artifacts = state
        .evaluation_artifact_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    if snapshot.used_evaluation_artifact_ids != used_artifacts {
        return Err("snapshot evaluation artifact set differs from the transition state".into());
    }
    if snapshot.reserved_calibration_groups
        != state
            .calibration_groups
            .values()
            .cloned()
            .collect::<Vec<_>>()
        || snapshot.reserved_holdout_groups
            != state.holdout_groups.values().cloned().collect::<Vec<_>>()
    {
        return Err("snapshot split registrations differ from the immutable envelope".into());
    }
    Ok(())
}

/// Independently derive and audit the logical lifecycle represented by a
/// SIM-REL-034 snapshot.
///
/// Unlike the SIM-REL-035 offline verifier, this function does not invoke the
/// live ledger's constructor, transition methods, perturbation code, or policy
/// replay implementation. It validates only logical transition and record
/// consistency. A successful receipt must be combined with physical replay,
/// provenance validation, and external split-registry evidence before being
/// treated as a scientific qualification.
pub fn audit_model_family_transitions_independently(
    snapshot: &ModelFamilyLedgerSnapshot,
) -> Result<IndependentTransitionAuditReceipt, TransitionOracleError> {
    let mut state = TransitionState::from_snapshot(snapshot)
        .map_err(|reason| fail(None, TransitionOraclePhase::SnapshotEnvelope, reason))?;

    for (index, sequenced) in snapshot.events.iter().enumerate() {
        let expected_sequence = u64::try_from(index).map_err(|_| {
            fail(
                None,
                TransitionOraclePhase::EventSequence,
                "event index exceeds u64",
            )
        })?;
        if sequenced.sequence != expected_sequence {
            return Err(fail(
                Some(sequenced.sequence),
                TransitionOraclePhase::EventSequence,
                format!(
                    "expected sequence {expected_sequence}, found {}",
                    sequenced.sequence
                ),
            ));
        }
        let expected_revision = match &sequenced.event {
            ModelFamilyLedgerEvent::CounterexampleAdmitted(record) => record.revision_before,
            _ => sequenced.event.family_revision(),
        };
        if expected_revision < state.revision
            || expected_revision > state.revision.saturating_add(1)
        {
            return Err(fail(
                Some(sequenced.sequence),
                TransitionOraclePhase::EventSequence,
                "event revision is not adjacent to the currently reconstructed family revision",
            ));
        }
        verify_event(
            &mut state,
            sequenced,
            snapshot.replay_context.development_budgets,
        )?;
    }

    compare_final_snapshot(
        snapshot,
        &state,
        snapshot.replay_context.development_budgets,
    )
    .map_err(|reason| fail(None, TransitionOraclePhase::FinalSnapshotEquality, reason))?;

    let event_count = u64::try_from(snapshot.events.len()).map_err(|_| {
        fail(
            None,
            TransitionOraclePhase::EventSequence,
            "event count exceeds u64",
        )
    })?;
    let development_world_ids = state.development_world_ids.iter().cloned().collect();
    let consumed_holdout_group_ids = state.consumed_holdout_group_ids.iter().cloned().collect();
    Ok(IndependentTransitionAuditReceipt {
        event_count,
        counterexample_admission_count: state.counts[0],
        policy_replan_count: state.counts[1],
        final_policy_seal_count: state.counts[2],
        sealed_holdout_evaluation_count: state.counts[3],
        final_family_revision: state.revision,
        development_world_ids,
        consumed_holdout_group_ids,
        active_policy_identity: state.active_policy.map(|policy| policy.identity),
        frozen_policy_identity: state.frozen_policy.map(|policy| policy.identity),
        logical_snapshot_exactly_reconstructed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::super::counterexample_ledger::{FamilyReplanRecord, SealedHoldoutEvaluationRecord};
    use super::super::counterexample_ledger::{ModelFamilyLedger, SealedHoldoutAcceptanceCriteria};
    use super::super::robust_finite_horizon::DecisionModelWorld;
    use super::super::{DecisionLossTable, ObservationProtocol};
    use super::*;
    use crate::{ActionRealizationKernel, BeliefMass, KernelRow, LatentState, TransitionOutcome};

    fn world(id: &str) -> DecisionModelWorld {
        let state = LatentState {
            hypothesis: 0,
            physical_state: 0,
            sensor_state: 0,
            apparatus_state: 0,
        };
        DecisionModelWorld {
            id: id.into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["observe".into()],
                realized_action_ids: vec!["executed".into()],
                readback_ids: vec!["ok".into()],
                measurement_ids: vec!["value".into()],
                initial_belief: vec![BeliefMass {
                    state,
                    mass: Rational::one(),
                }],
                rows: vec![KernelRow {
                    state,
                    command: 0,
                    outcomes: vec![TransitionOutcome {
                        probability: Rational::one(),
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: Some(0),
                        hazard: false,
                    }],
                }],
            },
        }
    }

    fn budgets() -> DecisionBudgets {
        DecisionBudgets {
            max_hazard_probability: Rational::zero(),
            max_abstention_probability: Rational::zero(),
        }
    }

    fn metrics(world_id: &str) -> Vec<WorldDecisionObjectives> {
        vec![WorldDecisionObjectives {
            world_id: world_id.into(),
            expected_experiment_cost: Rational::zero(),
            expected_terminal_loss: Rational::zero(),
            hazard_probability: Rational::zero(),
            abstention_probability: Rational::zero(),
        }]
    }

    fn initial_ledger() -> ModelFamilyLedger {
        let loss_table = DecisionLossTable {
            decision_ids: vec!["claim".into()],
            losses: vec![vec![vec![vec![vec![Rational::zero()]]]]],
            abstention_loss: Rational::one(),
        };
        ModelFamilyLedger::new(
            vec![world("nominal")],
            vec![CalibrationGroupRegistration {
                family_group_id: "cal-family".into(),
                world_ids: vec!["cal-world".into()],
                manifest_id: "cal-manifest".into(),
            }],
            vec![SealedHoldoutGroupRegistration {
                family_group_id: "sealed-family".into(),
                world_ids: vec!["holdout-world".into()],
                manifest_id: "sealed-manifest".into(),
            }],
            "policy-v1".into(),
            DecisionPlan::TerminalDecision { decision: Some(0) },
            "nominal",
            &loss_table,
            ObservationProtocol::fully_observed(),
            &[Rational::zero()],
            Rational::one(),
            0,
            budgets(),
        )
        .unwrap()
    }

    fn initial_snapshot() -> ModelFamilyLedgerSnapshot {
        initial_ledger().snapshot()
    }

    fn replan_event(sequence: u64) -> SequencedLedgerEvent {
        SequencedLedgerEvent {
            sequence,
            event: ModelFamilyLedgerEvent::PolicyReplanned(FamilyReplanRecord {
                event_receipt_id: "replan-1".into(),
                family_revision: 0,
                policy_identity: "policy-v2".into(),
                plan: DecisionPlan::TerminalDecision { decision: Some(0) },
                replayed_world_ids: vec!["nominal".into()],
                per_world_metrics: metrics("nominal"),
            }),
        }
    }

    #[test]
    fn initial_snapshot_is_audited_without_calling_the_live_verifier() {
        let snapshot = initial_snapshot();
        let receipt = audit_model_family_transitions_independently(&snapshot).unwrap();
        assert_eq!(receipt.event_count, 0);
        assert_eq!(receipt.final_family_revision, 0);
        assert_eq!(receipt.active_policy_identity.as_deref(), Some("policy-v1"));
        assert!(receipt.logical_snapshot_exactly_reconstructed);
    }

    #[test]
    fn rejects_replan_without_prior_policy_invalidation() {
        let mut snapshot = initial_snapshot();
        snapshot.events.push(replan_event(0));
        let error = audit_model_family_transitions_independently(&snapshot).unwrap_err();
        assert_eq!(error.phase, TransitionOraclePhase::PolicyReplan);
        assert_eq!(error.event_sequence, Some(0));
    }

    #[test]
    fn rejects_event_sequence_gaps_before_transition_evaluation() {
        let mut snapshot = initial_snapshot();
        snapshot.events.push(replan_event(1));
        let error = audit_model_family_transitions_independently(&snapshot).unwrap_err();
        assert_eq!(error.phase, TransitionOraclePhase::EventSequence);
        assert_eq!(error.event_sequence, Some(1));
    }

    #[test]
    fn rejects_forged_final_active_policy_state() {
        let mut snapshot = initial_snapshot();
        snapshot.active_policy_identity = Some("forged-policy".into());
        let error = audit_model_family_transitions_independently(&snapshot).unwrap_err();
        assert_eq!(error.phase, TransitionOraclePhase::FinalSnapshotEquality);
    }

    #[test]
    fn rejects_duplicate_split_group_registration() {
        let mut snapshot = initial_snapshot();
        snapshot
            .reserved_holdout_groups
            .push(snapshot.reserved_holdout_groups[0].clone());
        let error = audit_model_family_transitions_independently(&snapshot).unwrap_err();
        assert_eq!(error.phase, TransitionOraclePhase::SnapshotEnvelope);
    }

    #[test]
    fn audits_a_real_seal_and_successful_holdout_transcript_against_both_verifiers() {
        let mut ledger = initial_ledger();
        ledger
            .seal_final_policy("seal-real".into(), 0, "policy-v1".into())
            .unwrap();
        let criteria = SealedHoldoutAcceptanceCriteria {
            max_worst_case_total_loss: Rational::zero(),
            max_worst_case_terminal_loss: Rational::zero(),
            budgets: budgets(),
        };
        let evaluation = ledger
            .record_sealed_holdout_evaluation(
                "holdout-real".into(),
                "sealed-family".into(),
                0,
                "policy-v1".into(),
                &[world("holdout-world")],
                criteria,
                "artifact-real".into(),
            )
            .unwrap();
        assert!(evaluation.replay_succeeded);
        assert!(evaluation.acceptance_criteria_satisfied);

        let snapshot = ledger.snapshot();
        let independent = audit_model_family_transitions_independently(&snapshot).unwrap();
        let replay =
            super::super::counterexample_ledger::verify_model_family_ledger_snapshot(&snapshot)
                .unwrap();
        assert_eq!(independent.event_count, 2);
        assert_eq!(independent.final_policy_seal_count, 1);
        assert_eq!(independent.sealed_holdout_evaluation_count, 1);
        assert!(independent.logical_snapshot_exactly_reconstructed);
        assert_eq!(replay.event_count, 2);
        assert!(replay.snapshot_exactly_reconstructed);
    }

    #[test]
    fn accepts_seal_then_failed_holdout_as_consumed_terminal_evaluation() {
        let mut snapshot = initial_snapshot();
        let policy_plan = DecisionPlan::TerminalDecision { decision: Some(0) };
        let policy_metrics = metrics("nominal");
        let seal = FinalPolicySealRecord {
            event_receipt_id: "seal-event".into(),
            family_revision: 0,
            policy_identity: "policy-v1".into(),
            plan: policy_plan.clone(),
            per_world_metrics: policy_metrics.clone(),
        };
        let criteria = SealedHoldoutAcceptanceCriteria {
            max_worst_case_total_loss: Rational::zero(),
            max_worst_case_terminal_loss: Rational::zero(),
            budgets: budgets(),
        };
        let evaluation = SealedHoldoutEvaluationRecord {
            event_receipt_id: "holdout-event".into(),
            family_group_id: "sealed-family".into(),
            manifest_id: "sealed-manifest".into(),
            family_revision: 0,
            policy_identity: "policy-v1".into(),
            world_ids: vec!["holdout-world".into()],
            holdout_world_snapshots: vec![world("holdout-world")],
            evaluation_artifact_id: "evaluation-artifact".into(),
            criteria,
            replay_succeeded: false,
            replay_error: Some("synthetic forced replay failure".into()),
            per_world_metrics: Vec::new(),
            worst_case_total_loss: None,
            worst_case_terminal_loss: None,
            acceptance_criteria_satisfied: false,
        };
        snapshot.events = vec![
            SequencedLedgerEvent {
                sequence: 0,
                event: ModelFamilyLedgerEvent::FinalPolicySealed(seal),
            },
            SequencedLedgerEvent {
                sequence: 1,
                event: ModelFamilyLedgerEvent::SealedHoldoutEvaluated(evaluation),
            },
        ];
        snapshot.frozen_policy_identity = Some("policy-v1".into());
        snapshot.frozen_policy_plan = Some(policy_plan);
        snapshot.frozen_policy_metrics = Some(policy_metrics);
        snapshot.frozen_policy_revision = Some(0);
        snapshot.consumed_holdout_group_ids = vec!["sealed-family".into()];
        snapshot.used_evaluation_artifact_ids = vec!["evaluation-artifact".into()];

        let receipt = audit_model_family_transitions_independently(&snapshot).unwrap();
        assert_eq!(receipt.final_policy_seal_count, 1);
        assert_eq!(receipt.sealed_holdout_evaluation_count, 1);
        assert_eq!(receipt.consumed_holdout_group_ids, vec!["sealed-family"]);
    }

    #[test]
    fn rejects_post_seal_replan_even_if_policy_identity_is_new() {
        let mut snapshot = initial_snapshot();
        let policy_plan = DecisionPlan::TerminalDecision { decision: Some(0) };
        let policy_metrics = metrics("nominal");
        snapshot.events.push(SequencedLedgerEvent {
            sequence: 0,
            event: ModelFamilyLedgerEvent::FinalPolicySealed(FinalPolicySealRecord {
                event_receipt_id: "seal-event".into(),
                family_revision: 0,
                policy_identity: "policy-v1".into(),
                plan: policy_plan.clone(),
                per_world_metrics: policy_metrics.clone(),
            }),
        });
        snapshot.events.push(replan_event(1));
        snapshot.frozen_policy_identity = Some("policy-v1".into());
        snapshot.frozen_policy_plan = Some(policy_plan);
        snapshot.frozen_policy_metrics = Some(policy_metrics);
        snapshot.frozen_policy_revision = Some(0);
        let error = audit_model_family_transitions_independently(&snapshot).unwrap_err();
        assert_eq!(error.phase, TransitionOraclePhase::PolicyReplan);
        assert_eq!(error.event_sequence, Some(1));
    }
}
