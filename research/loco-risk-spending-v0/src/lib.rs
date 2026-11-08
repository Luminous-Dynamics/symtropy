// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite-horizon Bonferroni alpha allocation and irreversible risk accounting.
//!
//! A family-wise alpha budget is preallocated over every horizon step and
//! every declared physical constraint before actions are selected. Skipped
//! allocations are recorded as unused, not redistributed. Once an action is
//! committed, its allocation remains reserved until a matching, verified
//! execution receipt consumes it. This crate is an accounting primitive; it
//! does not validate the marginal probability model or certify physical safety.

use std::collections::BTreeSet;

pub const CONSTRAINT_COUNT: usize = 5;
pub const MIN_FAMILYWISE_ALPHA_PPB: u64 = 50;
pub const MAX_FAMILYWISE_ALPHA_PPB: u64 = 500_000_000;
pub const MAX_HORIZON_STEPS: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskConstraint {
    Energy,
    SupportMargin,
    PeakNormalForce,
    ImpactImpulse,
    ActuatorLoadFraction,
}

impl RiskConstraint {
    pub fn index(self) -> usize {
        match self {
            Self::Energy => 0,
            Self::SupportMargin => 1,
            Self::PeakNormalForce => 2,
            Self::ImpactImpulse => 3,
            Self::ActuatorLoadFraction => 4,
        }
    }

    pub fn all() -> [Self; CONSTRAINT_COUNT] {
        [
            Self::Energy,
            Self::SupportMargin,
            Self::PeakNormalForce,
            Self::ImpactImpulse,
            Self::ActuatorLoadFraction,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskPlanIdentity {
    pub episode_id: String,
    pub risk_plan_id: String,
    pub model_digest: String,
    pub actuator_model_digest: String,
    pub operating_domain_digest: String,
    pub calibration_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskPlanError {
    EmptyIdentity,
    InvalidHorizon,
    HorizonTooLarge,
    InvalidFamilywiseAlpha,
    InsufficientRiskResolution,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizonRiskPlan {
    identity: RiskPlanIdentity,
    horizon_steps: u32,
    familywise_alpha_ppb: u64,
    step_allocations_ppb: Vec<[u64; CONSTRAINT_COUNT]>,
}

impl HorizonRiskPlan {
    pub fn new(
        identity: RiskPlanIdentity,
        horizon_steps: u32,
        familywise_alpha_ppb: u64,
    ) -> Result<Self, RiskPlanError> {
        if [
            identity.episode_id.as_str(),
            identity.risk_plan_id.as_str(),
            identity.model_digest.as_str(),
            identity.operating_domain_digest.as_str(),
            identity.calibration_digest.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(RiskPlanError::EmptyIdentity);
        }
        if horizon_steps == 0 {
            return Err(RiskPlanError::InvalidHorizon);
        }
        if horizon_steps > MAX_HORIZON_STEPS {
            return Err(RiskPlanError::HorizonTooLarge);
        }
        if !(MIN_FAMILYWISE_ALPHA_PPB..=MAX_FAMILYWISE_ALPHA_PPB).contains(&familywise_alpha_ppb) {
            return Err(RiskPlanError::InvalidFamilywiseAlpha);
        }

        let event_count = u64::from(horizon_steps)
            .checked_mul(CONSTRAINT_COUNT as u64)
            .ok_or(RiskPlanError::ArithmeticOverflow)?;
        if familywise_alpha_ppb < event_count {
            return Err(RiskPlanError::InsufficientRiskResolution);
        }

        let base = familywise_alpha_ppb / event_count;
        let remainder = familywise_alpha_ppb % event_count;
        let capacity =
            usize::try_from(horizon_steps).map_err(|_| RiskPlanError::ArithmeticOverflow)?;
        let mut step_allocations_ppb = vec![[base; CONSTRAINT_COUNT]; capacity];

        // Deterministically distribute indivisible ppb units in canonical
        // (step index, constraint index) order.
        for flat_index in 0..remainder {
            let step = usize::try_from(flat_index / CONSTRAINT_COUNT as u64)
                .map_err(|_| RiskPlanError::ArithmeticOverflow)?;
            let constraint = usize::try_from(flat_index % CONSTRAINT_COUNT as u64)
                .map_err(|_| RiskPlanError::ArithmeticOverflow)?;
            step_allocations_ppb[step][constraint] += 1;
        }

        let plan = Self {
            identity,
            horizon_steps,
            familywise_alpha_ppb,
            step_allocations_ppb,
        };
        if plan.total_allocated_alpha_ppb() != familywise_alpha_ppb
            || plan
                .step_allocations_ppb
                .iter()
                .flatten()
                .any(|alpha| *alpha == 0)
        {
            return Err(RiskPlanError::InsufficientRiskResolution);
        }
        Ok(plan)
    }

    /// Construct an immutable, predeclared nonuniform allocation profile.
    ///
    /// This permits phase- or constraint-weighted risk allocation while
    /// preserving one exact family-wise horizon budget. The vector is frozen
    /// into the plan; the ledger provides no post-outcome reallocation path.
    pub fn from_preallocated_allocations(
        identity: RiskPlanIdentity,
        step_allocations_ppb: Vec<[u64; CONSTRAINT_COUNT]>,
    ) -> Result<Self, RiskPlanError> {
        if [
            identity.episode_id.as_str(),
            identity.risk_plan_id.as_str(),
            identity.model_digest.as_str(),
            identity.operating_domain_digest.as_str(),
            identity.calibration_digest.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(RiskPlanError::EmptyIdentity);
        }

        if step_allocations_ppb.is_empty() {
            return Err(RiskPlanError::InvalidHorizon);
        }
        if step_allocations_ppb.len() > MAX_HORIZON_STEPS as usize {
            return Err(RiskPlanError::HorizonTooLarge);
        }
        if step_allocations_ppb
            .iter()
            .flatten()
            .any(|alpha| *alpha == 0)
        {
            return Err(RiskPlanError::InsufficientRiskResolution);
        }

        let familywise_alpha_ppb = step_allocations_ppb
            .iter()
            .flat_map(|allocation| allocation.iter())
            .try_fold(0_u64, |sum, alpha| sum.checked_add(*alpha))
            .ok_or(RiskPlanError::ArithmeticOverflow)?;
        if !(MIN_FAMILYWISE_ALPHA_PPB..=MAX_FAMILYWISE_ALPHA_PPB).contains(&familywise_alpha_ppb) {
            return Err(RiskPlanError::InvalidFamilywiseAlpha);
        }

        let horizon_steps = u32::try_from(step_allocations_ppb.len())
            .map_err(|_| RiskPlanError::ArithmeticOverflow)?;
        Ok(Self {
            identity,
            horizon_steps,
            familywise_alpha_ppb,
            step_allocations_ppb,
        })
    }

    pub fn identity(&self) -> &RiskPlanIdentity {
        &self.identity
    }

    pub fn horizon_steps(&self) -> u32 {
        self.horizon_steps
    }

    pub fn familywise_alpha_ppb(&self) -> u64 {
        self.familywise_alpha_ppb
    }

    pub fn allocation_for_step(
        &self,
        step_index: u32,
    ) -> Result<[u64; CONSTRAINT_COUNT], LedgerError> {
        self.step_allocations_ppb
            .get(step_index as usize)
            .copied()
            .ok_or(LedgerError::StepOutOfRange)
    }

    pub fn allocation_for(
        &self,
        step_index: u32,
        constraint: RiskConstraint,
    ) -> Result<u64, LedgerError> {
        let allocation = self.allocation_for_step(step_index)?;
        Ok(allocation[constraint.index()])
    }

    pub fn total_allocated_alpha_ppb(&self) -> u64 {
        self.step_allocations_ppb
            .iter()
            .flat_map(|allocation| allocation.iter())
            .copied()
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAdmissionReceipt {
    pub receipt_id: String,
    pub episode_id: String,
    pub risk_plan_id: String,
    pub step_index: u32,
    pub action_id: String,
    pub action_digest: String,
    pub model_digest: String,
    pub actuator_model_digest: String,
    pub operating_domain_digest: String,
    pub calibration_digest: String,
    pub allocated_alpha_ppb: [u64; CONSTRAINT_COUNT],
    pub committed_at_ns: u64,
    /// Trust input from the independent one-step chance/robust gate.
    pub admission_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepExecutionReceipt {
    pub receipt_id: String,
    pub admission_receipt_id: String,
    pub episode_id: String,
    pub risk_plan_id: String,
    pub step_index: u32,
    pub action_id: String,
    pub delivered_action_id: String,
    pub action_digest: String,
    pub model_digest: String,
    pub actuator_model_digest: String,
    pub operating_domain_digest: String,
    pub calibration_digest: String,
    pub measured_actuation_digest: String,
    pub executed_at_ns: u64,
    /// Trust input from the action-delivery verifier.
    pub execution_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepSkipReceipt {
    pub receipt_id: String,
    pub episode_id: String,
    pub risk_plan_id: String,
    pub step_index: u32,
    /// Required when reconciling an Unknown committed step; absent for a
    /// pre-commit skip.
    pub admission_receipt_id: Option<String>,
    pub reason: String,
    pub evidence_digest: String,
    pub skipped_at_ns: u64,
    /// Requires independent evidence that no physical action was delivered.
    pub no_execution_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepUnknownReceipt {
    pub receipt_id: String,
    pub episode_id: String,
    pub risk_plan_id: String,
    pub step_index: u32,
    pub admission_receipt_id: String,
    pub reason: String,
    pub evidence_digest: String,
    pub recorded_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Committed {
        admission_receipt_id: String,
        action_id: String,
        action_digest: String,
        committed_at_ns: u64,
    },
    Consumed {
        admission_receipt_id: String,
        execution_receipt_id: String,
        action_id: String,
        action_digest: String,
        executed_at_ns: u64,
    },
    Unknown {
        unknown_receipt_id: String,
        admission_receipt_id: String,
        action_id: String,
        action_digest: String,
        committed_at_ns: u64,
        reason: String,
        recorded_at_ns: u64,
    },
    Skipped {
        skip_receipt_id: String,
        reason: String,
        skipped_at_ns: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    EmptyIdentity,
    StepOutOfRange,
    StepOutOfOrder,
    StepAlreadyResolved,
    StepNotCommitted,
    AdmissionReceiptNotVerified,
    ExecutionReceiptNotVerified,
    SkipReceiptNotVerified,
    UnknownIdentityMismatch,
    AdmissionIdentityMismatch,
    ExecutionIdentityMismatch,
    SkipIdentityMismatch,
    AllocationMismatch,
    DuplicateReceipt,
    ExecutionBeforeCommit,
    NonMonotonicEventTime,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskLedgerSummary {
    pub familywise_alpha_ppb: u64,
    /// Risk amount reserved by admitted/committed actions whose execution
    /// receipt is not yet closed.
    pub committed_reserved_alpha_ppb: u64,
    /// Risk amount attached to actions with verified delivery evidence.
    pub consumed_alpha_ppb: u64,
    /// Committed allocations whose execution/delivery status is unresolved.
    /// These remain locked and are never treated as unused or reusable.
    pub unknown_locked_alpha_ppb: u64,
    /// Allocations for steps explicitly skipped before an action was committed.
    /// They remain unused and cannot be transferred to a future step.
    pub skipped_unused_alpha_ppb: u64,
    /// Allocations belonging to future, unresolved steps.
    pub pending_preallocated_alpha_ppb: u64,
    pub committed_step_count: u32,
    pub consumed_step_count: u32,
    pub skipped_step_count: u32,
    pub unknown_step_count: u32,
    pub resolved_step_count: u32,
    pub next_step_index: u32,
}

impl RiskLedgerSummary {
    pub fn reserved_or_consumed_alpha_ppb(&self) -> Result<u64, LedgerError> {
        self.committed_reserved_alpha_ppb
            .checked_add(self.consumed_alpha_ppb)
            .and_then(|value| value.checked_add(self.unknown_locked_alpha_ppb))
            .ok_or(LedgerError::ArithmeticOverflow)
    }

    pub fn unexposed_preallocated_alpha_ppb(&self) -> Result<u64, LedgerError> {
        self.skipped_unused_alpha_ppb
            .checked_add(self.pending_preallocated_alpha_ppb)
            .ok_or(LedgerError::ArithmeticOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizonRiskLedger {
    plan: HorizonRiskPlan,
    statuses: Vec<StepStatus>,
    next_step_index: u32,
    last_event_time_ns: Option<u64>,
    seen_receipt_ids: BTreeSet<String>,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

impl HorizonRiskLedger {
    pub fn new(plan: HorizonRiskPlan) -> Self {
        let statuses = vec![StepStatus::Pending; plan.horizon_steps as usize];
        Self {
            plan,
            statuses,
            next_step_index: 0,
            last_event_time_ns: None,
            seen_receipt_ids: BTreeSet::new(),
        }
    }

    pub fn plan(&self) -> &HorizonRiskPlan {
        &self.plan
    }

    pub fn status(&self, step_index: u32) -> Result<&StepStatus, LedgerError> {
        self.statuses
            .get(step_index as usize)
            .ok_or(LedgerError::StepOutOfRange)
    }

    fn validate_next_step_index(&self, step_index: u32) -> Result<(), LedgerError> {
        if step_index >= self.plan.horizon_steps {
            return Err(LedgerError::StepOutOfRange);
        }
        if step_index != self.next_step_index {
            return Err(LedgerError::StepOutOfOrder);
        }
        Ok(())
    }

    fn validate_next_pending_step(&self, step_index: u32) -> Result<(), LedgerError> {
        self.validate_next_step_index(step_index)?;
        if self.statuses[step_index as usize] != StepStatus::Pending {
            return Err(LedgerError::StepAlreadyResolved);
        }
        Ok(())
    }

    fn validate_monotonic_time(&self, event_time_ns: u64) -> Result<(), LedgerError> {
        if self
            .last_event_time_ns
            .is_some_and(|last| event_time_ns < last)
        {
            return Err(LedgerError::NonMonotonicEventTime);
        }
        Ok(())
    }

    fn insert_receipt_id(&mut self, receipt_id: &str) -> Result<(), LedgerError> {
        if !nonblank(receipt_id) {
            return Err(LedgerError::EmptyIdentity);
        }
        if !self.seen_receipt_ids.insert(receipt_id.to_owned()) {
            return Err(LedgerError::DuplicateReceipt);
        }
        Ok(())
    }

    /// Commit a one-step admitted action under the immutable allocation for
    /// this exact episode/step. This reserves risk before physical execution.
    pub fn commit_step(&mut self, receipt: StepAdmissionReceipt) -> Result<(), LedgerError> {
        self.validate_next_pending_step(receipt.step_index)?;
        self.validate_monotonic_time(receipt.committed_at_ns)?;
        if [
            receipt.receipt_id.as_str(),
            receipt.episode_id.as_str(),
            receipt.risk_plan_id.as_str(),
            receipt.action_id.as_str(),
            receipt.action_digest.as_str(),
            receipt.model_digest.as_str(),
            receipt.actuator_model_digest.as_str(),
            receipt.operating_domain_digest.as_str(),
            receipt.calibration_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(LedgerError::EmptyIdentity);
        }
        if !receipt.admission_verified {
            return Err(LedgerError::AdmissionReceiptNotVerified);
        }
        let identity = self.plan.identity();
        if receipt.episode_id != identity.episode_id
            || receipt.risk_plan_id != identity.risk_plan_id
            || receipt.model_digest != identity.model_digest
            || receipt.actuator_model_digest != identity.actuator_model_digest
            || receipt.operating_domain_digest != identity.operating_domain_digest
            || receipt.calibration_digest != identity.calibration_digest
        {
            return Err(LedgerError::AdmissionIdentityMismatch);
        }
        if receipt.allocated_alpha_ppb != self.plan.allocation_for_step(receipt.step_index)? {
            return Err(LedgerError::AllocationMismatch);
        }

        self.insert_receipt_id(&receipt.receipt_id)?;
        self.statuses[receipt.step_index as usize] = StepStatus::Committed {
            admission_receipt_id: receipt.receipt_id,
            action_id: receipt.action_id,
            action_digest: receipt.action_digest,
            committed_at_ns: receipt.committed_at_ns,
        };
        self.last_event_time_ns = Some(receipt.committed_at_ns);
        Ok(())
    }

    /// Consume a committed allocation only when a verified execution receipt
    /// binds the delivered action to the preceding admission receipt.
    pub fn consume_step(&mut self, receipt: StepExecutionReceipt) -> Result<(), LedgerError> {
        self.validate_next_step_index(receipt.step_index)?;
        let status = self.statuses[receipt.step_index as usize].clone();
        let (admission_receipt_id, action_id, action_digest, committed_at_ns) = match status {
            StepStatus::Committed {
                admission_receipt_id,
                action_id,
                action_digest,
                committed_at_ns,
            }
            | StepStatus::Unknown {
                admission_receipt_id,
                action_id,
                action_digest,
                committed_at_ns,
                ..
            } => (
                admission_receipt_id,
                action_id,
                action_digest,
                committed_at_ns,
            ),
            _ => return Err(LedgerError::StepNotCommitted),
        };
        if [
            receipt.receipt_id.as_str(),
            receipt.admission_receipt_id.as_str(),
            receipt.episode_id.as_str(),
            receipt.risk_plan_id.as_str(),
            receipt.action_id.as_str(),
            receipt.delivered_action_id.as_str(),
            receipt.action_digest.as_str(),
            receipt.model_digest.as_str(),
            receipt.actuator_model_digest.as_str(),
            receipt.operating_domain_digest.as_str(),
            receipt.calibration_digest.as_str(),
            receipt.measured_actuation_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(LedgerError::EmptyIdentity);
        }
        if !receipt.execution_verified {
            return Err(LedgerError::ExecutionReceiptNotVerified);
        }
        if receipt.episode_id != self.plan.identity.episode_id
            || receipt.risk_plan_id != self.plan.identity.risk_plan_id
            || receipt.admission_receipt_id != admission_receipt_id
            || receipt.action_id != action_id
            || receipt.action_digest != action_digest
            || receipt.model_digest != self.plan.identity.model_digest
            || receipt.actuator_model_digest != self.plan.identity.actuator_model_digest
            || receipt.operating_domain_digest != self.plan.identity.operating_domain_digest
            || receipt.calibration_digest != self.plan.identity.calibration_digest
        {
            return Err(LedgerError::ExecutionIdentityMismatch);
        }
        if receipt.executed_at_ns < committed_at_ns {
            return Err(LedgerError::ExecutionBeforeCommit);
        }
        self.validate_monotonic_time(receipt.executed_at_ns)?;

        self.insert_receipt_id(&receipt.receipt_id)?;
        self.statuses[receipt.step_index as usize] = StepStatus::Consumed {
            admission_receipt_id,
            execution_receipt_id: receipt.receipt_id,
            action_id,
            action_digest,
            executed_at_ns: receipt.executed_at_ns,
        };
        self.last_event_time_ns = Some(receipt.executed_at_ns);
        self.next_step_index += 1;
        Ok(())
    }

    /// Mark a committed step Unknown when delivery/execution provenance is
    /// unresolved. Its allocation remains locked, the horizon cannot advance,
    /// and it cannot be classified as unused until independently reconciled.
    pub fn mark_unknown(&mut self, receipt: StepUnknownReceipt) -> Result<(), LedgerError> {
        self.validate_next_step_index(receipt.step_index)?;
        self.validate_monotonic_time(receipt.recorded_at_ns)?;
        if [
            receipt.receipt_id.as_str(),
            receipt.episode_id.as_str(),
            receipt.risk_plan_id.as_str(),
            receipt.admission_receipt_id.as_str(),
            receipt.reason.as_str(),
            receipt.evidence_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(LedgerError::EmptyIdentity);
        }
        if receipt.episode_id != self.plan.identity.episode_id
            || receipt.risk_plan_id != self.plan.identity.risk_plan_id
        {
            return Err(LedgerError::UnknownIdentityMismatch);
        }
        let status = self.statuses[receipt.step_index as usize].clone();
        let StepStatus::Committed {
            admission_receipt_id,
            action_id,
            action_digest,
            committed_at_ns,
        } = status
        else {
            return Err(LedgerError::StepNotCommitted);
        };
        if receipt.admission_receipt_id != admission_receipt_id
            || receipt.recorded_at_ns < committed_at_ns
        {
            return Err(LedgerError::UnknownIdentityMismatch);
        }
        self.insert_receipt_id(&receipt.receipt_id)?;
        self.statuses[receipt.step_index as usize] = StepStatus::Unknown {
            unknown_receipt_id: receipt.receipt_id,
            admission_receipt_id,
            action_id,
            action_digest,
            committed_at_ns,
            reason: receipt.reason,
            recorded_at_ns: receipt.recorded_at_ns,
        };
        self.last_event_time_ns = Some(receipt.recorded_at_ns);
        Ok(())
    }

    /// Mark a step unused only when an independent receipt establishes that no
    /// action was delivered. The allocation is retained as skipped-unused and
    /// is never redistributed to another step.
    pub fn skip_step(&mut self, receipt: StepSkipReceipt) -> Result<(), LedgerError> {
        self.validate_next_step_index(receipt.step_index)?;
        self.validate_monotonic_time(receipt.skipped_at_ns)?;
        if [
            receipt.receipt_id.as_str(),
            receipt.episode_id.as_str(),
            receipt.risk_plan_id.as_str(),
            receipt.reason.as_str(),
            receipt.evidence_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(LedgerError::EmptyIdentity);
        }
        if !receipt.no_execution_verified {
            return Err(LedgerError::SkipReceiptNotVerified);
        }
        if receipt.episode_id != self.plan.identity.episode_id
            || receipt.risk_plan_id != self.plan.identity.risk_plan_id
        {
            return Err(LedgerError::SkipIdentityMismatch);
        }
        match &self.statuses[receipt.step_index as usize] {
            StepStatus::Pending if receipt.admission_receipt_id.is_none() => {}
            StepStatus::Unknown {
                admission_receipt_id,
                ..
            } if receipt.admission_receipt_id.as_deref() == Some(admission_receipt_id.as_str()) => {
            }
            _ => return Err(LedgerError::StepAlreadyResolved),
        }

        self.insert_receipt_id(&receipt.receipt_id)?;
        self.statuses[receipt.step_index as usize] = StepStatus::Skipped {
            skip_receipt_id: receipt.receipt_id,
            reason: receipt.reason,
            skipped_at_ns: receipt.skipped_at_ns,
        };
        self.last_event_time_ns = Some(receipt.skipped_at_ns);
        self.next_step_index += 1;
        Ok(())
    }

    pub fn summary(&self) -> Result<RiskLedgerSummary, LedgerError> {
        let mut summary = RiskLedgerSummary {
            familywise_alpha_ppb: self.plan.familywise_alpha_ppb,
            committed_reserved_alpha_ppb: 0,
            consumed_alpha_ppb: 0,
            unknown_locked_alpha_ppb: 0,
            skipped_unused_alpha_ppb: 0,
            pending_preallocated_alpha_ppb: 0,
            committed_step_count: 0,
            consumed_step_count: 0,
            skipped_step_count: 0,
            unknown_step_count: 0,
            resolved_step_count: 0,
            next_step_index: self.next_step_index,
        };
        for (step_index, status) in self.statuses.iter().enumerate() {
            let amount = self.plan.step_allocations_ppb[step_index]
                .iter()
                .sum::<u64>();
            match status {
                StepStatus::Pending => {
                    summary.pending_preallocated_alpha_ppb = summary
                        .pending_preallocated_alpha_ppb
                        .checked_add(amount)
                        .ok_or(LedgerError::ArithmeticOverflow)?;
                }
                StepStatus::Committed { .. } => {
                    summary.committed_reserved_alpha_ppb = summary
                        .committed_reserved_alpha_ppb
                        .checked_add(amount)
                        .ok_or(LedgerError::ArithmeticOverflow)?;
                    summary.committed_step_count += 1;
                }
                StepStatus::Consumed { .. } => {
                    summary.consumed_alpha_ppb = summary
                        .consumed_alpha_ppb
                        .checked_add(amount)
                        .ok_or(LedgerError::ArithmeticOverflow)?;
                    summary.consumed_step_count += 1;
                    summary.resolved_step_count += 1;
                }
                StepStatus::Unknown { .. } => {
                    summary.unknown_locked_alpha_ppb = summary
                        .unknown_locked_alpha_ppb
                        .checked_add(amount)
                        .ok_or(LedgerError::ArithmeticOverflow)?;
                    summary.unknown_step_count += 1;
                }
                StepStatus::Skipped { .. } => {
                    summary.skipped_unused_alpha_ppb = summary
                        .skipped_unused_alpha_ppb
                        .checked_add(amount)
                        .ok_or(LedgerError::ArithmeticOverflow)?;
                    summary.skipped_step_count += 1;
                    summary.resolved_step_count += 1;
                }
            }
        }

        let accounted = summary
            .committed_reserved_alpha_ppb
            .checked_add(summary.consumed_alpha_ppb)
            .and_then(|value| value.checked_add(summary.unknown_locked_alpha_ppb))
            .and_then(|value| value.checked_add(summary.skipped_unused_alpha_ppb))
            .and_then(|value| value.checked_add(summary.pending_preallocated_alpha_ppb))
            .ok_or(LedgerError::ArithmeticOverflow)?;
        if accounted != summary.familywise_alpha_ppb {
            return Err(LedgerError::ArithmeticOverflow);
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> RiskPlanIdentity {
        RiskPlanIdentity {
            episode_id: "episode-01".to_owned(),
            risk_plan_id: "risk-plan-01".to_owned(),
            model_digest: "model-v1".to_owned(),
            actuator_model_digest: "actuator-model-v1".to_owned(),
            operating_domain_digest: "rough-terrain-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
        }
    }

    fn plan(horizon: u32, alpha_ppb: u64) -> HorizonRiskPlan {
        HorizonRiskPlan::new(identity(), horizon, alpha_ppb).unwrap()
    }

    fn admission(
        plan: &HorizonRiskPlan,
        step: u32,
        suffix: &str,
        committed_at_ns: u64,
    ) -> StepAdmissionReceipt {
        StepAdmissionReceipt {
            receipt_id: format!("admission-{suffix}"),
            episode_id: "episode-01".to_owned(),
            risk_plan_id: "risk-plan-01".to_owned(),
            step_index: step,
            action_id: format!("action-{suffix}"),
            action_digest: format!("action-digest-{suffix}"),
            model_digest: "model-v1".to_owned(),
            actuator_model_digest: "actuator-model-v1".to_owned(),
            operating_domain_digest: "rough-terrain-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            allocated_alpha_ppb: plan.allocation_for_step(step).unwrap(),
            committed_at_ns,
            admission_verified: true,
        }
    }

    fn execution(
        admission: &StepAdmissionReceipt,
        suffix: &str,
        executed_at_ns: u64,
    ) -> StepExecutionReceipt {
        StepExecutionReceipt {
            receipt_id: format!("execution-{suffix}"),
            admission_receipt_id: admission.receipt_id.clone(),
            episode_id: admission.episode_id.clone(),
            risk_plan_id: admission.risk_plan_id.clone(),
            step_index: admission.step_index,
            action_id: admission.action_id.clone(),
            delivered_action_id: format!("delivered-{suffix}"),
            action_digest: admission.action_digest.clone(),
            model_digest: admission.model_digest.clone(),
            actuator_model_digest: admission.actuator_model_digest.clone(),
            operating_domain_digest: admission.operating_domain_digest.clone(),
            calibration_digest: admission.calibration_digest.clone(),
            measured_actuation_digest: format!("measured-actuation-{suffix}"),
            executed_at_ns,
            execution_verified: true,
        }
    }

    fn skip(step: u32, suffix: &str, skipped_at_ns: u64) -> StepSkipReceipt {
        StepSkipReceipt {
            receipt_id: format!("skip-{suffix}"),
            episode_id: "episode-01".to_owned(),
            risk_plan_id: "risk-plan-01".to_owned(),
            step_index: step,
            admission_receipt_id: None,
            reason: "candidate rejected before delivery".to_owned(),
            evidence_digest: format!("no-delivery-evidence-{suffix}"),
            skipped_at_ns,
            no_execution_verified: true,
        }
    }

    fn unknown(
        admission: &StepAdmissionReceipt,
        suffix: &str,
        recorded_at_ns: u64,
    ) -> StepUnknownReceipt {
        StepUnknownReceipt {
            receipt_id: format!("unknown-{suffix}"),
            episode_id: admission.episode_id.clone(),
            risk_plan_id: admission.risk_plan_id.clone(),
            step_index: admission.step_index,
            admission_receipt_id: admission.receipt_id.clone(),
            reason: "delivery acknowledgement missing".to_owned(),
            evidence_digest: format!("unknown-evidence-{suffix}"),
            recorded_at_ns,
        }
    }

    #[test]
    fn one_step_horizon_matches_one_step_five_constraint_budget() {
        let risk_plan = plan(1, 50_000_000);
        assert_eq!(risk_plan.allocation_for_step(0).unwrap(), [10_000_000; 5]);
        assert_eq!(risk_plan.total_allocated_alpha_ppb(), 50_000_000);
    }

    #[test]
    fn two_step_horizon_splits_family_alpha_across_ten_events() {
        let risk_plan = plan(2, 50_000_000);
        assert_eq!(risk_plan.allocation_for_step(0).unwrap(), [5_000_000; 5]);
        assert_eq!(risk_plan.allocation_for_step(1).unwrap(), [5_000_000; 5]);
        assert_eq!(risk_plan.total_allocated_alpha_ppb(), 50_000_000);
    }

    #[test]
    fn three_step_uneven_ppb_budget_distributes_remainder_canonically() {
        let risk_plan = plan(3, 50_000_002);
        assert_eq!(risk_plan.allocation_for_step(0).unwrap(), [3_333_334; 5]);
        assert_eq!(
            risk_plan.allocation_for_step(1).unwrap(),
            [3_333_334, 3_333_334, 3_333_333, 3_333_333, 3_333_333]
        );
        assert_eq!(risk_plan.total_allocated_alpha_ppb(), 50_000_002);
    }

    #[test]
    fn horizon_ten_allocation_sums_exactly() {
        let risk_plan = plan(10, 50_000_000);
        assert_eq!(risk_plan.total_allocated_alpha_ppb(), 50_000_000);
        assert!(
            risk_plan
                .step_allocations_ppb
                .iter()
                .flatten()
                .all(|value| *value > 0)
        );
    }

    #[test]
    fn allocation_rejects_zero_resolution_instead_of_zero_alpha_events() {
        assert_eq!(
            HorizonRiskPlan::new(identity(), 11, 50),
            Err(RiskPlanError::InsufficientRiskResolution)
        );
    }

    #[test]
    fn invalid_horizon_or_alpha_is_rejected() {
        assert_eq!(
            HorizonRiskPlan::new(identity(), 0, 50_000_000),
            Err(RiskPlanError::InvalidHorizon)
        );
        assert_eq!(
            HorizonRiskPlan::new(identity(), 1, 0),
            Err(RiskPlanError::InvalidFamilywiseAlpha)
        );
        assert_eq!(
            HorizonRiskPlan::new(identity(), 1, 500_000_001),
            Err(RiskPlanError::InvalidFamilywiseAlpha)
        );
        assert_eq!(
            HorizonRiskPlan::new(identity(), MAX_HORIZON_STEPS + 1, 500_000_000),
            Err(RiskPlanError::HorizonTooLarge)
        );
    }

    #[test]
    fn allocation_is_exact_for_many_horizons() {
        for horizon in [1, 2, 3, 10, 127, 1_000] {
            let risk_plan = plan(horizon, 500_000_000);
            assert_eq!(risk_plan.total_allocated_alpha_ppb(), 500_000_000);
            assert_eq!(risk_plan.step_allocations_ppb.len(), horizon as usize);
            assert!(
                risk_plan
                    .step_allocations_ppb
                    .iter()
                    .flatten()
                    .all(|value| *value > 0)
            );
        }
    }

    #[test]
    fn each_step_has_five_canonical_constraint_allocations() {
        let risk_plan = plan(2, 50_000_000);
        let allocation = risk_plan.allocation_for_step(1).unwrap();
        assert_eq!(allocation.len(), CONSTRAINT_COUNT);
        assert_eq!(
            risk_plan
                .allocation_for(1, RiskConstraint::SupportMargin)
                .unwrap(),
            allocation[RiskConstraint::SupportMargin.index()]
        );
    }

    #[test]
    fn skipped_step_does_not_transfer_alpha_to_later_steps() {
        let risk_plan = plan(2, 50_000_000);
        let original_step_one = risk_plan.allocation_for_step(1).unwrap();
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        ledger.skip_step(skip(0, "zero", 1_000)).unwrap();
        assert_eq!(
            ledger.plan().allocation_for_step(1).unwrap(),
            original_step_one
        );
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.skipped_unused_alpha_ppb, 25_000_000);
        assert_eq!(summary.pending_preallocated_alpha_ppb, 25_000_000);
        assert_eq!(summary.consumed_alpha_ppb, 0);
    }

    #[test]
    fn committed_and_consumed_risk_are_accounted_separately() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let admission_receipt = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(admission_receipt.clone()).unwrap();
        let committed = ledger.summary().unwrap();
        assert_eq!(committed.committed_reserved_alpha_ppb, 25_000_000);
        assert_eq!(committed.pending_preallocated_alpha_ppb, 25_000_000);
        assert_eq!(committed.consumed_alpha_ppb, 0);

        ledger
            .consume_step(execution(&admission_receipt, "zero", 1_100))
            .unwrap();
        let consumed = ledger.summary().unwrap();
        assert_eq!(consumed.committed_reserved_alpha_ppb, 0);
        assert_eq!(consumed.consumed_alpha_ppb, 25_000_000);
        assert_eq!(consumed.pending_preallocated_alpha_ppb, 25_000_000);
    }

    #[test]
    fn steps_must_be_resolved_in_order() {
        let risk_plan = plan(3, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let out_of_order = admission(&risk_plan, 1, "one", 1_000);
        assert_eq!(
            ledger.commit_step(out_of_order),
            Err(LedgerError::StepOutOfOrder)
        );
    }

    #[test]
    fn duplicate_skip_and_unverified_no_delivery_receipt_are_rejected() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let mut unverified = skip(0, "zero", 1_000);
        unverified.no_execution_verified = false;
        assert_eq!(
            ledger.skip_step(unverified),
            Err(LedgerError::SkipReceiptNotVerified)
        );
        let valid = skip(0, "zero", 1_000);
        ledger.skip_step(valid.clone()).unwrap();
        assert_eq!(ledger.skip_step(valid), Err(LedgerError::StepOutOfOrder));
    }

    #[test]
    fn wrong_episode_action_plan_or_calibration_receipt_cannot_commit() {
        let risk_plan = plan(2, 50_000_000);
        let mut wrong_episode = admission(&risk_plan, 0, "wrong-episode", 1_000);
        wrong_episode.episode_id = "other-episode".to_owned();
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        assert_eq!(
            ledger.commit_step(wrong_episode),
            Err(LedgerError::AdmissionIdentityMismatch)
        );

        let mut wrong_calibration = admission(&risk_plan, 0, "wrong-calibration", 1_000);
        wrong_calibration.calibration_digest = "other-calibration".to_owned();
        assert_eq!(
            ledger.commit_step(wrong_calibration),
            Err(LedgerError::AdmissionIdentityMismatch)
        );
    }

    #[test]
    fn wrong_execution_action_digest_or_receipt_cannot_consume() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let committed = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(committed.clone()).unwrap();
        let mut wrong = execution(&committed, "zero", 1_100);
        wrong.action_digest = "different-action".to_owned();
        assert_eq!(
            ledger.consume_step(wrong),
            Err(LedgerError::ExecutionIdentityMismatch)
        );
        assert_eq!(
            ledger.summary().unwrap().committed_reserved_alpha_ppb,
            25_000_000
        );
    }

    #[test]
    fn unverified_execution_receipt_keeps_budget_reserved_and_blocks_progress() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let committed = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(committed.clone()).unwrap();
        let mut unverified = execution(&committed, "zero", 1_100);
        unverified.execution_verified = false;
        assert_eq!(
            ledger.consume_step(unverified),
            Err(LedgerError::ExecutionReceiptNotVerified)
        );
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.committed_reserved_alpha_ppb, 25_000_000);
        assert_eq!(summary.consumed_alpha_ppb, 0);
        let later = admission(&risk_plan, 1, "one", 1_200);
        assert_eq!(ledger.commit_step(later), Err(LedgerError::StepOutOfOrder));
    }

    #[test]
    fn duplicate_receipt_cannot_double_consume_or_replay() {
        let risk_plan = plan(1, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let committed = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(committed.clone()).unwrap();
        let executed = execution(&committed, "zero", 1_100);
        ledger.consume_step(executed.clone()).unwrap();
        assert_eq!(
            ledger.consume_step(executed),
            Err(LedgerError::StepOutOfOrder)
        );
        assert_eq!(ledger.summary().unwrap().consumed_alpha_ppb, 50_000_000);
    }

    #[test]
    fn execution_before_commit_or_non_monotonic_time_is_rejected() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let committed = admission(&risk_plan, 0, "zero", 2_000);
        ledger.commit_step(committed.clone()).unwrap();
        assert_eq!(
            ledger.consume_step(execution(&committed, "zero", 1_999)),
            Err(LedgerError::ExecutionBeforeCommit)
        );
    }

    #[test]
    fn skipped_or_consumed_alpha_is_never_added_to_future_allocations() {
        let risk_plan = plan(3, 50_000_000);
        let step_two_before = risk_plan.allocation_for_step(2).unwrap();
        let step_zero_total = risk_plan
            .allocation_for_step(0)
            .unwrap()
            .iter()
            .sum::<u64>();
        let step_one_total = risk_plan
            .allocation_for_step(1)
            .unwrap()
            .iter()
            .sum::<u64>();
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        ledger.skip_step(skip(0, "zero", 1_000)).unwrap();
        let committed = admission(&risk_plan, 1, "one", 1_100);
        ledger.commit_step(committed.clone()).unwrap();
        ledger
            .consume_step(execution(&committed, "one", 1_200))
            .unwrap();
        assert_eq!(risk_plan.allocation_for_step(2).unwrap(), step_two_before);
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.skipped_unused_alpha_ppb, step_zero_total);
        assert_eq!(summary.consumed_alpha_ppb, step_one_total);
        assert_eq!(
            summary.pending_preallocated_alpha_ppb,
            50_000_000 - step_zero_total - step_one_total
        );
    }

    #[test]
    fn skipped_step_requires_monotonic_event_time() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        ledger.skip_step(skip(0, "zero", 2_000)).unwrap();
        let later_skip = skip(1, "one", 1_999);
        assert_eq!(
            ledger.skip_step(later_skip),
            Err(LedgerError::NonMonotonicEventTime)
        );
    }

    #[test]
    fn unknown_execution_locks_alpha_and_blocks_horizon_progress() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let admitted = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(admitted.clone()).unwrap();
        ledger
            .mark_unknown(unknown(&admitted, "zero", 1_050))
            .unwrap();

        assert!(matches!(
            ledger.status(0).unwrap(),
            StepStatus::Unknown { .. }
        ));
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.unknown_locked_alpha_ppb, 25_000_000);
        assert_eq!(summary.committed_reserved_alpha_ppb, 0);
        assert_eq!(summary.consumed_alpha_ppb, 0);
        assert_eq!(
            summary.unexposed_preallocated_alpha_ppb().unwrap(),
            25_000_000
        );
        assert_eq!(
            summary.reserved_or_consumed_alpha_ppb().unwrap(),
            25_000_000
        );
        assert_eq!(
            ledger.commit_step(admission(&risk_plan, 1, "one", 1_100)),
            Err(LedgerError::StepOutOfOrder)
        );
    }

    #[test]
    fn unknown_step_can_only_reconcile_to_consumed_with_matching_execution_receipt() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let admitted = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(admitted.clone()).unwrap();
        ledger
            .mark_unknown(unknown(&admitted, "zero", 1_050))
            .unwrap();

        let mut mismatched = execution(&admitted, "zero", 1_100);
        mismatched.actuator_model_digest = "other-actuator-model".to_owned();
        assert_eq!(
            ledger.consume_step(mismatched),
            Err(LedgerError::ExecutionIdentityMismatch)
        );
        assert_eq!(
            ledger.summary().unwrap().unknown_locked_alpha_ppb,
            25_000_000
        );

        ledger
            .consume_step(execution(&admitted, "zero", 1_100))
            .unwrap();
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.unknown_locked_alpha_ppb, 0);
        assert_eq!(summary.consumed_alpha_ppb, 25_000_000);
        assert_eq!(summary.resolved_step_count, 1);
        assert_eq!(summary.next_step_index, 1);
    }

    #[test]
    fn unknown_step_can_only_reconcile_to_skipped_with_matching_no_delivery_receipt() {
        let risk_plan = plan(2, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let admitted = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(admitted.clone()).unwrap();
        ledger
            .mark_unknown(unknown(&admitted, "zero", 1_050))
            .unwrap();

        let mut no_delivery = skip(0, "zero", 1_100);
        no_delivery.admission_receipt_id = Some(admitted.receipt_id.clone());
        ledger.skip_step(no_delivery).unwrap();
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.unknown_locked_alpha_ppb, 0);
        assert_eq!(summary.skipped_unused_alpha_ppb, 25_000_000);
        assert_eq!(summary.pending_preallocated_alpha_ppb, 25_000_000);
        assert_eq!(summary.resolved_step_count, 1);
    }

    #[test]
    fn unknown_receipt_must_match_episode_and_admission() {
        let risk_plan = plan(1, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let admitted = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(admitted.clone()).unwrap();

        let mut wrong_episode = unknown(&admitted, "wrong-episode", 1_050);
        wrong_episode.episode_id = "other-episode".to_owned();
        assert_eq!(
            ledger.mark_unknown(wrong_episode),
            Err(LedgerError::UnknownIdentityMismatch)
        );

        let mut wrong_admission = unknown(&admitted, "wrong-admission", 1_050);
        wrong_admission.admission_receipt_id = "other-admission".to_owned();
        assert_eq!(
            ledger.mark_unknown(wrong_admission),
            Err(LedgerError::UnknownIdentityMismatch)
        );
    }

    #[test]
    fn unknown_steps_are_not_counted_as_unused_or_resolved() {
        let risk_plan = plan(1, 50_000_000);
        let mut ledger = HorizonRiskLedger::new(risk_plan.clone());
        let admitted = admission(&risk_plan, 0, "zero", 1_000);
        ledger.commit_step(admitted.clone()).unwrap();
        ledger
            .mark_unknown(unknown(&admitted, "zero", 1_050))
            .unwrap();
        let summary = ledger.summary().unwrap();
        assert_eq!(summary.unknown_step_count, 1);
        assert_eq!(summary.skipped_unused_alpha_ppb, 0);
        assert_eq!(summary.resolved_step_count, 0);
        assert_eq!(
            summary.pending_preallocated_alpha_ppb
                + summary.committed_reserved_alpha_ppb
                + summary.consumed_alpha_ppb
                + summary.unknown_locked_alpha_ppb
                + summary.skipped_unused_alpha_ppb,
            summary.familywise_alpha_ppb
        );
    }

    #[test]
    fn preallocated_nonuniform_profile_preserves_exact_step_and_constraint_values() {
        let profile = vec![
            [6_000_000, 5_000_000, 4_000_000, 3_000_000, 2_000_000],
            [1_000_000, 2_000_000, 3_000_000, 4_000_000, 5_000_000],
        ];
        let risk_plan =
            HorizonRiskPlan::from_preallocated_allocations(identity(), profile.clone()).unwrap();

        assert_eq!(risk_plan.horizon_steps(), 2);
        assert_eq!(risk_plan.familywise_alpha_ppb(), 35_000_000);
        assert_eq!(risk_plan.allocation_for_step(0).unwrap(), profile[0]);
        assert_eq!(risk_plan.allocation_for_step(1).unwrap(), profile[1]);
        assert_eq!(risk_plan.total_allocated_alpha_ppb(), 35_000_000);
    }

    #[test]
    fn preallocated_nonuniform_profile_rejects_empty_zero_or_invalid_budget() {
        assert_eq!(
            HorizonRiskPlan::from_preallocated_allocations(identity(), Vec::new()),
            Err(RiskPlanError::InvalidHorizon)
        );
        assert_eq!(
            HorizonRiskPlan::from_preallocated_allocations(identity(), vec![[10, 10, 10, 10, 0]],),
            Err(RiskPlanError::InsufficientRiskResolution)
        );
        assert_eq!(
            HorizonRiskPlan::from_preallocated_allocations(identity(), vec![[1, 1, 1, 1, 1]],),
            Err(RiskPlanError::InvalidFamilywiseAlpha)
        );
    }

    #[test]
    fn plan_allocation_is_deterministic_for_same_identity_and_budget() {
        let left = plan(127, 123_456_789);
        let right = plan(127, 123_456_789);
        assert_eq!(left, right);
    }
}
