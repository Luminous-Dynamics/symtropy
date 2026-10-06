// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic kernel for space-bootstrap and industrial-closure reasoning.
//!
//! This crate intentionally has no Bevy, Holochain, Symthaea, or external
//! simulation dependencies. It turns industrial closure into explicit,
//! replayable state and graph calculations that higher layers can consume.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum dependency depth evaluated by the recursive closure resolver.
///
/// A deterministic ceiling prevents pathological authored topology from turning
/// into an unbounded process-stack obligation. Exceeding the ceiling is treated
/// as unresolved closure data, not as a successful or partial resolution.
const MAX_DEPENDENCY_RESOLUTION_DEPTH: usize = 1024;

/// State of a terminal dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyClass {
    LocalClosed,
    LocalOpen,
    ImportedConsumable,
    ImportedDurable,
    ReplaceableBySubstitution,
    Unknown,
}

impl DependencyClass {
    /// Whether this dependency can count as locally closed.
    pub const fn is_closed(self) -> bool {
        matches!(self, Self::LocalClosed | Self::ReplaceableBySubstitution)
    }
}

/// A terminal dependency in the industrial closure graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub id: String,
    pub class: DependencyClass,
}

impl Dependency {
    #[must_use]
    pub fn new(id: impl Into<String>, class: DependencyClass) -> Self {
        Self {
            id: id.into(),
            class,
        }
    }
}

/// A capability with a criticality weight and dependency edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub id: String,
    pub critical_weight: u64,
    pub dependencies: Vec<String>,
}

impl Capability {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        critical_weight: u64,
        dependencies: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            id: id.into(),
            critical_weight,
            dependencies: dependencies.into_iter().map(Into::into).collect(),
        }
    }
}

/// Evaluation of one capability against the dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAssessment {
    pub id: String,
    pub closed: bool,
    pub unresolved_dependencies: Vec<String>,
    pub cycle_detected: bool,
}

/// Aggregate deterministic closure report.
///
/// Ratios are represented as integer parts-per-million values so replay and
/// cross-platform comparisons do not depend on floating-point arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureReport {
    pub assessments: Vec<CapabilityAssessment>,
    pub weighted_critical_closed: u64,
    pub weighted_critical_total: u64,
    pub critical_closure_ppm: u64,
    pub mass_closure_ppm: u64,
    pub valid: bool,
    pub definition_errors: Vec<String>,
}

impl ClosureReport {
    #[must_use]
    pub const fn fully_closed(&self) -> bool {
        self.valid
            && self.weighted_critical_total > 0
            && self.weighted_critical_closed == self.weighted_critical_total
    }

    #[must_use]
    pub fn definition_errors(&self) -> &[String] {
        &self.definition_errors
    }
}

/// A dependency that constrains one or more failed capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyBlocker {
    pub id: String,
    pub weight: u64,
    pub affected_capabilities: Vec<String>,
}

/// Deterministic dependency graph.
#[derive(Debug, Default, Clone)]
pub struct DependencyGraph {
    capabilities: BTreeMap<String, Capability>,
    dependencies: BTreeMap<String, Dependency>,
    definition_errors: Vec<String>,
}

impl DependencyGraph {
    #[must_use]
    pub fn new(
        capabilities: impl IntoIterator<Item = Capability>,
        dependencies: impl IntoIterator<Item = Dependency>,
    ) -> Self {
        let mut capability_map = BTreeMap::new();
        let mut dependency_map = BTreeMap::new();
        let mut definition_errors = BTreeSet::new();

        for capability in capabilities {
            if capability.id.is_empty() {
                definition_errors.insert("empty capability ID".to_string());
                continue;
            }

            let id = capability.id.clone();
            if capability_map.contains_key(&id) {
                definition_errors.insert(format!("duplicate capability ID: {id}"));
                continue;
            }

            capability_map.insert(id, capability);
        }

        for dependency in dependencies {
            if dependency.id.is_empty() {
                definition_errors.insert("empty dependency ID".to_string());
                continue;
            }

            let id = dependency.id.clone();
            if dependency_map.contains_key(&id) {
                definition_errors.insert(format!("duplicate dependency ID: {id}"));
                continue;
            }

            dependency_map.insert(id, dependency);
        }

        for id in capability_map.keys() {
            if dependency_map.contains_key(id) {
                definition_errors.insert(format!("capability/dependency ID collision: {id}"));
            }
        }

        Self {
            capabilities: capability_map,
            dependencies: dependency_map,
            definition_errors: definition_errors.into_iter().collect(),
        }
    }

    /// Evaluate every capability in deterministic key order.
    #[must_use]
    pub fn evaluate(&self, local_mass_g: u64, imported_mass_g: u64) -> ClosureReport {
        let mut memo = BTreeMap::new();
        let mut assessments = Vec::with_capacity(self.capabilities.len());

        for id in self.capabilities.keys() {
            let (closed, mut unresolved, cycle_detected) =
                self.resolve(id, &mut memo, &mut Vec::new());

            unresolved.sort();
            unresolved.dedup();

            assessments.push(CapabilityAssessment {
                id: id.clone(),
                closed,
                unresolved_dependencies: unresolved,
                cycle_detected,
            });
        }

        let mut report_definition_errors = self.definition_errors.clone();

        let weighted_critical_total = self
            .capabilities
            .values()
            .try_fold(0_u64, |sum, capability| {
                sum.checked_add(capability.critical_weight)
            });

        if weighted_critical_total.is_none() {
            report_definition_errors.push("critical weight sum overflow".to_string());
        }

        let weighted_critical_closed = assessments
            .iter()
            .filter(|assessment| assessment.closed)
            .filter_map(|assessment| self.capabilities.get(&assessment.id))
            .try_fold(0_u64, |sum, capability| {
                sum.checked_add(capability.critical_weight)
            });

        if weighted_critical_closed.is_none() {
            report_definition_errors.push("closed critical weight sum overflow".to_string());
        }

        if assessments.iter().any(|assessment| {
            assessment
                .unresolved_dependencies
                .iter()
                .any(|dependency| dependency.starts_with("resolution-depth-exceeded:"))
        }) {
            report_definition_errors.push(format!(
                "dependency resolution depth exceeded: {MAX_DEPENDENCY_RESOLUTION_DEPTH}"
            ));
        }

        let mass_total = local_mass_g.checked_add(imported_mass_g);
        if mass_total.is_none() {
            report_definition_errors.push("mass total overflow".to_string());
        }

        if !report_definition_errors.is_empty() {
            for assessment in &mut assessments {
                assessment.closed = false;
                assessment.unresolved_dependencies.extend(
                    report_definition_errors
                        .iter()
                        .map(|error| format!("definition:{error}")),
                );
                assessment.unresolved_dependencies.sort();
                assessment.unresolved_dependencies.dedup();
            }
        }

        let valid = report_definition_errors.is_empty();
        let weighted_critical_total = weighted_critical_total.unwrap_or(0);
        let weighted_critical_closed = weighted_critical_closed.unwrap_or(0);

        ClosureReport {
            assessments,
            weighted_critical_closed,
            weighted_critical_total,
            critical_closure_ppm: if valid {
                ratio_ppm(weighted_critical_closed, weighted_critical_total)
            } else {
                0
            },
            mass_closure_ppm: if valid {
                ratio_ppm(local_mass_g, mass_total.unwrap_or(0))
            } else {
                0
            },
            valid,
            definition_errors: report_definition_errors,
        }
    }

    /// Rank unresolved dependencies by the criticality weight they block.
    ///
    /// Weight aggregation is checked rather than saturated: an overflow is
    /// invalid qualification data, not a reason to silently cap the blocker.
    pub fn rank_blockers(&self, report: &ClosureReport) -> Result<Vec<DependencyBlocker>, String> {
        if !report.valid {
            return Err("cannot rank blockers for invalid closure report".to_string());
        }

        let mut blockers: BTreeMap<String, (u64, BTreeSet<String>)> = BTreeMap::new();

        for assessment in &report.assessments {
            if assessment.closed {
                continue;
            }

            let Some(capability) = self.capabilities.get(&assessment.id) else {
                continue;
            };

            for dependency in &assessment.unresolved_dependencies {
                let entry = blockers
                    .entry(dependency.clone())
                    .or_insert_with(|| (0, BTreeSet::new()));
                entry.0 = entry
                    .0
                    .checked_add(capability.critical_weight)
                    .ok_or_else(|| {
                        format!("blocker weight overflow for dependency: {dependency}")
                    })?;
                entry.1.insert(capability.id.clone());
            }
        }

        let mut ranked = blockers
            .into_iter()
            .map(|(id, (weight, affected_capabilities))| DependencyBlocker {
                id,
                weight,
                affected_capabilities: affected_capabilities.into_iter().collect(),
            })
            .collect::<Vec<_>>();

        ranked.sort_by(|left, right| {
            right
                .weight
                .cmp(&left.weight)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(ranked)
    }

    fn resolve(
        &self,
        id: &str,
        memo: &mut BTreeMap<String, bool>,
        stack: &mut Vec<String>,
    ) -> (bool, Vec<String>, bool) {
        if let Some(closed) = memo.get(id) {
            if *closed {
                return (true, Vec::new(), false);
            }
        }

        if stack.iter().any(|entry| entry == id) {
            return (false, vec![id.to_string()], true);
        }

        if let Some(dependency) = self.dependencies.get(id) {
            let closed = dependency.class.is_closed();
            if closed {
                memo.insert(id.to_string(), true);
            }
            return (
                closed,
                if closed {
                    Vec::new()
                } else {
                    vec![id.to_string()]
                },
                false,
            );
        }

        let Some(capability) = self.capabilities.get(id) else {
            return (false, vec![id.to_string()], false);
        };

        if stack.len() >= MAX_DEPENDENCY_RESOLUTION_DEPTH {
            return (
                false,
                vec![format!("resolution-depth-exceeded:{MAX_DEPENDENCY_RESOLUTION_DEPTH}")],
                false,
            );
        }

        stack.push(id.to_string());

        let mut closed = true;
        let mut unresolved = Vec::new();
        let mut cycle_detected = false;

        for dependency_id in &capability.dependencies {
            let (dependency_closed, mut missing, cycle) =
                if self.capabilities.contains_key(dependency_id) {
                    self.resolve(dependency_id, memo, stack)
                } else if let Some(dependency) = self.dependencies.get(dependency_id) {
                    (
                        dependency.class.is_closed(),
                        if dependency.class.is_closed() {
                            Vec::new()
                        } else {
                            vec![dependency_id.clone()]
                        },
                        false,
                    )
                } else {
                    (false, vec![dependency_id.clone()], false)
                };

            closed &= dependency_closed;
            unresolved.append(&mut missing);
            cycle_detected |= cycle;
        }

        stack.pop();

        if closed {
            memo.insert(id.to_string(), true);
        }

        (closed, unresolved, cycle_detected)
    }
}

/// Evidence-backed industrial closure stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClosureStage {
    Seed,
    Repair,
    Feedstock,
    Structural,
    Machine,
    Control,
    Factory,
    Ecological,
    Expansion,
}

/// Capabilities that must be closed to earn a stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRequirement {
    pub stage: ClosureStage,
    pub capabilities: Vec<String>,
}

impl StageRequirement {
    #[must_use]
    pub fn new(
        stage: ClosureStage,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            stage,
            capabilities: capabilities.into_iter().map(Into::into).collect(),
        }
    }
}

/// Return the highest stage for which all declared capabilities are closed.
#[must_use]
pub fn highest_closed_stage(
    report: &ClosureReport,
    requirements: &[StageRequirement],
) -> Option<ClosureStage> {
    if !report.valid {
        return None;
    }

    let closed = report
        .assessments
        .iter()
        .filter(|assessment| assessment.closed)
        .map(|assessment| assessment.id.as_str())
        .collect::<BTreeSet<_>>();

    let mut ordered = requirements.to_vec();
    ordered.sort_by_key(|requirement| requirement.stage);

    if ordered
        .iter()
        .any(|requirement| requirement.capabilities.is_empty())
        || ordered
            .windows(2)
            .any(|window| window[0].stage == window[1].stage)
    {
        return None;
    }

    let mut highest = None;
    for requirement in ordered {
        let satisfied = requirement
            .capabilities
            .iter()
            .all(|capability| closed.contains(capability.as_str()));

        if !satisfied {
            break;
        }

        highest = Some(requirement.stage);
    }

    highest
}

/// Lifecycle state of a process execution authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionState {
    /// Budget capacity is reserved; executable receipts additionally require a
    /// matching concrete source reservation before physical execution is allowed.
    Pending,
    /// The funded execution completed and its causal ledger effects were settled.
    Committed,
    /// The funded execution was explicitly canceled and its capacity was restored.
    Aborted,
}

impl ExecutionState {
    /// Whether a lifecycle record may legally advance from `self` to `next`.
    ///
    /// There is no terminal-to-terminal or terminal-to-pending transition, and
    /// repeating the same state is not treated as an idempotent transition.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Committed) | (Self::Pending, Self::Aborted)
        )
    }
}
/// Immutable lifecycle record for one execution authorization.
///
/// The terminal record retains the exact receipt that reached the terminal
/// state, so recovery and audit never have to infer causal identity from ledger
/// effects alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionLifecycleRecord {
    execution_id: String,
    state: ExecutionState,
    receipt: ProcessExecutionReceipt,
}

impl ExecutionLifecycleRecord {
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    #[must_use]
    pub const fn state(&self) -> ExecutionState {
        self.state
    }

    #[must_use]
    pub fn receipt(&self) -> &ProcessExecutionReceipt {
        &self.receipt
    }
}

/// Consumable authorization budget for one deterministic execution scope.
///
/// Feedstock and energy are reserved exactly once when an execution receipt is
/// minted. The reservation state is private so callers cannot restore capacity
/// without creating a new budget scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionBudget {
    available_feed_mass_g: u64,
    available_energy_units: u64,
    reserved_executions: BTreeMap<String, ProcessExecutionReceipt>,
    terminal_execution_states: BTreeMap<String, ExecutionLifecycleRecord>,
}

impl ExecutionBudget {
    #[must_use]
    pub const fn new(available_feed_mass_g: u64, available_energy_units: u64) -> Self {
        Self {
            available_feed_mass_g,
            available_energy_units,
            reserved_executions: BTreeMap::new(),
            terminal_execution_states: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn available_feed_mass_g(&self) -> u64 {
        self.available_feed_mass_g
    }

    #[must_use]
    pub const fn available_energy_units(&self) -> u64 {
        self.available_energy_units
    }

    /// Return a deterministic SHA-256 commitment of the complete budget state.
    #[must_use]
    pub fn state_commitment(&self) -> String {
        let mut hasher = CommitmentHasher::new("symtropy.execution.budget.v1");
        hasher.u64(self.available_feed_mass_g);
        hasher.u64(self.available_energy_units);
        hasher.receipt_map(&self.reserved_executions);

        hasher.u64(self.terminal_execution_states.len() as u64);
        for (execution_id, record) in &self.terminal_execution_states {
            hasher.string(execution_id);
            hasher.string(record.execution_id());
            hasher.byte(match record.state() {
                ExecutionState::Pending => 0,
                ExecutionState::Committed => 1,
                ExecutionState::Aborted => 2,
            });
            hasher.receipt(record.receipt());
        }

        hasher.finish()
    }

    /// Reserve capacity for the complete receipt, not just its quantities.
    ///
    /// Binding the reservation to the immutable receipt prevents a future
    /// caller or internal refactor from swapping process, batch, sequence, node,
    /// or run parameters while retaining the same funded execution identity.
    fn reserve(&mut self, receipt: &ProcessExecutionReceipt) -> Result<(), String> {
        let execution_id = receipt.execution_id();

        if self.reserved_executions.contains_key(execution_id)
            || self.terminal_execution_states.contains_key(execution_id)
        {
            return Err(format!("execution ID already authorized: {execution_id}"));
        }

        if receipt.feed_mass_g() > self.available_feed_mass_g {
            return Err(format!(
                "insufficient feedstock budget: required={}, available={}",
                receipt.feed_mass_g(),
                self.available_feed_mass_g
            ));
        }

        if receipt.energy_units() > self.available_energy_units {
            return Err(format!(
                "insufficient energy budget: required={}, available={}",
                receipt.energy_units(),
                self.available_energy_units
            ));
        }

        self.reserved_executions
            .insert(execution_id.to_string(), receipt.clone());
        self.available_feed_mass_g -= receipt.feed_mass_g();
        self.available_energy_units -= receipt.energy_units();

        Ok(())
    }

    fn reservation(&self, execution_id: &str) -> Option<&ProcessExecutionReceipt> {
        self.reserved_executions.get(execution_id)
    }

    /// Inspect a still-pending authorization for explicit recovery/rehydration.
    ///
    /// Returning the immutable stored receipt lets a higher layer recover an
    /// executable wrapper after the wrapper itself was dropped, without minting
    /// a new authorization or changing budget state.
    #[must_use]
    pub fn pending_receipt(&self, execution_id: &str) -> Option<&ProcessExecutionReceipt> {
        self.reservation(execution_id)
    }

    /// Return the complete lifecycle state of an execution.
    ///
    /// Pending is derived from the authoritative reservation map; terminal
    /// states are retained permanently so recovery code can distinguish commit
    /// from abort without inspecting ledger history.
    #[must_use]
    pub fn execution_state(&self, execution_id: &str) -> Option<ExecutionState> {
        if self.reserved_executions.contains_key(execution_id) {
            Some(ExecutionState::Pending)
        } else {
            self.terminal_execution_states
                .get(execution_id)
                .map(ExecutionLifecycleRecord::state)
        }
    }

    /// Return the exact lifecycle record, including the immutable receipt.
    #[must_use]
    pub fn execution_record(&self, execution_id: &str) -> Option<ExecutionLifecycleRecord> {
        if let Some(receipt) = self.reserved_executions.get(execution_id) {
            return Some(ExecutionLifecycleRecord {
                execution_id: receipt.execution_id().to_string(),
                state: ExecutionState::Pending,
                receipt: receipt.clone(),
            });
        }

        self.terminal_execution_states.get(execution_id).cloned()
    }

    fn settle(&mut self, execution_id: &str) -> Result<(), String> {
        let receipt = self
            .reserved_executions
            .remove(execution_id)
            .ok_or_else(|| format!("execution is not pending: {execution_id}"))?;

        self.terminal_execution_states.insert(
            execution_id.to_string(),
            ExecutionLifecycleRecord {
                execution_id: execution_id.to_string(),
                state: ExecutionState::Committed,
                receipt,
            },
        );
        Ok(())
    }

    /// Permanently record an explicit abort after restoring its reserved capacity.
    fn abort(&mut self, execution_id: &str) -> Result<(), String> {
        let receipt = self
            .reserved_executions
            .remove(execution_id)
            .ok_or_else(|| format!("execution is not pending: {execution_id}"))?;

        self.available_feed_mass_g = self
            .available_feed_mass_g
            .checked_add(receipt.feed_mass_g())
            .ok_or_else(|| "feedstock budget overflow during abort".to_string())?;
        self.available_energy_units = self
            .available_energy_units
            .checked_add(receipt.energy_units())
            .ok_or_else(|| "energy budget overflow during abort".to_string())?;

        self.terminal_execution_states.insert(
            execution_id.to_string(),
            ExecutionLifecycleRecord {
                execution_id: execution_id.to_string(),
                state: ExecutionState::Aborted,
                receipt,
            },
        );
        Ok(())
    }
}

/// A manufacturing process definition with explicit co-product accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionProcess {
    pub id: String,
    pub input_material: String,
    pub output_streams: Vec<String>,
    pub waste_stream: String,
}

impl ProductionProcess {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        input_material: impl Into<String>,
        output_streams: impl IntoIterator<Item = impl Into<String>>,
        waste_stream: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            input_material: input_material.into(),
            output_streams: output_streams.into_iter().map(Into::into).collect(),
            waste_stream: waste_stream.into(),
        }
    }

    /// Return a canonical SHA-256 commitment of the exact authored process definition.
    ///
    /// Output-stream order is retained deliberately: this binds durable execution
    /// history to the exact process configuration that was authorized, not merely
    /// to a process ID that could later be reused for a different definition.
    #[must_use]
    pub fn commitment(&self) -> String {
        let mut hasher = CommitmentHasher::new("symtropy.execution.process-definition.v1");
        hasher.string(&self.id);
        hasher.string(&self.input_material);
        hasher.u64(self.output_streams.len() as u64);
        for stream in &self.output_streams {
            hasher.string(stream);
        }
        hasher.string(&self.waste_stream);
        hasher.finish()
    }

    /// Validate that a process execution conforms to the declared process schema.
    pub fn validate_run(&self, run: &ProcessRun) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("process definition requires a non-empty process ID".to_string());
        }

        if self.input_material.is_empty() {
            return Err("process definition requires a non-empty input material".to_string());
        }

        if self.waste_stream.is_empty() {
            return Err("process definition requires a non-empty waste stream".to_string());
        }

        if self.output_streams.iter().any(String::is_empty) {
            return Err("process definition requires non-empty output stream IDs".to_string());
        }

        let mut declared = BTreeSet::new();
        for stream in &self.output_streams {
            if !declared.insert(stream.as_str()) {
                return Err(format!("duplicate output stream ID: {stream}"));
            }
        }

        if declared.contains(self.waste_stream.as_str()) {
            return Err(format!(
                "waste stream collides with output stream: {}",
                self.waste_stream
            ));
        }

        if run.process_id != self.id {
            return Err(format!(
                "process ID mismatch: expected={}, observed={}",
                self.id, run.process_id
            ));
        }

        if run.input_material != self.input_material {
            return Err(format!(
                "input material mismatch: expected={}, observed={}",
                self.input_material, run.input_material
            ));
        }

        if run.input_batch_id.is_empty() {
            return Err("process run requires a non-empty input batch ID".to_string());
        }

        if let Some(undeclared) = run
            .output_mass_g
            .keys()
            .find(|stream| !declared.contains(stream.as_str()))
        {
            return Err(format!("undeclared output stream: {undeclared}"));
        }

        run.validate_mass_balance()
    }

    /// Validate a process run against available feedstock and energy budgets.
    ///
    /// This keeps the kernel from treating a valid mass-balanced reaction as
    /// executable when its required feedstock or energy has not been provisioned.
    pub fn validate_run_against_budget(
        &self,
        run: &ProcessRun,
        available_feed_mass_g: u64,
        available_energy_units: u64,
    ) -> Result<(), String> {
        self.validate_run(run)?;

        if run.feed_mass_g > available_feed_mass_g {
            return Err(format!(
                "insufficient feedstock: required={}, available={available_feed_mass_g}",
                run.feed_mass_g
            ));
        }

        if run.energy_units > available_energy_units {
            return Err(format!(
                "insufficient energy: required={}, available={available_energy_units}",
                run.energy_units
            ));
        }

        Ok(())
    }

    /// Authorize one pending process execution against aggregate budgets and its concrete source batch.
    ///
    /// The physical source reservation is acquired before budget reservation. A
    /// failed budget reservation releases the source claim so the operation is
    /// atomic across both admission boundaries.
    ///
    /// This is the durability boundary for higher layers: the returned raw
    /// receipt proves the pending reservation, but it cannot materialize causal
    /// events or cross the commit/abort execution boundary. A durable adapter
    /// should persist this receipt and its reservation before calling
    /// `resume_pending_execution`.
    pub fn authorize_pending_execution_with_inventory(
        &self,
        execution_id: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
    ) -> Result<ProcessExecutionReceipt, String> {
        self.authorize_pending_execution_with_inventory_at_anchor(
            execution_id,
            first_inventory_sequence,
            energy_sequence,
            node_id,
            ExecutionStateAnchor::in_memory(),
            run,
            budget,
            inventory,
        )
    }

    /// Authorize one pending process execution against a caller-supplied state frontier.
    ///
    /// A durable adapter should derive the anchor from its independently verified
    /// journal/state head and persist the resulting receipt before activation.
    pub fn authorize_pending_execution_with_inventory_at_anchor(
        &self,
        execution_id: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        state_anchor: ExecutionStateAnchor,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
    ) -> Result<ProcessExecutionReceipt, String> {
        let execution_id = execution_id.into();
        let node_id = node_id.into();

        if execution_id.is_empty() {
            return Err("execution requires a non-empty execution ID".to_string());
        }
        if node_id.is_empty() {
            return Err("execution requires a non-empty energy node ID".to_string());
        }

        state_anchor.verify_state(budget, inventory)?;

        self.validate_run_against_budget(
            &run,
            budget.available_feed_mass_g,
            budget.available_energy_units,
        )?;

        let receipt = ProcessExecutionReceipt {
            execution_id: execution_id.clone(),
            process_id: self.id.clone(),
            input_batch_id: run.input_batch_id.clone(),
            waste_stream: self.waste_stream.clone(),
            first_inventory_sequence,
            energy_sequence,
            energy_node_id: node_id,
            state_anchor,
            run,
        };

        inventory.reserve_source_batch(
            receipt.execution_id(),
            receipt.input_batch_id(),
            receipt.feed_mass_g(),
        )?;

        if let Err(error) = budget.reserve(&receipt) {
            let _ = inventory.release_source_batch(receipt.execution_id());
            return Err(error);
        }

        Ok(receipt)
    }

    /// Restore an already-authorized pending execution from an exact persisted receipt.
    ///
    /// This replays the pending reservation transition without minting a new
    /// execution identity, sequence, or receipt. The supplied receipt must match
    /// this process definition, and the supplied budget/inventory must represent
    /// the state immediately before that pending transition.
    pub fn restore_pending_execution_with_inventory(
        &self,
        receipt: &ProcessExecutionReceipt,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
    ) -> Result<(), String> {
        self.restore_pending_execution_with_inventory_at_anchor(
            receipt,
            &ExecutionStateAnchor::in_memory(),
            budget,
            inventory,
        )
    }

    /// Restore an already-authorized pending execution against an exact state frontier.
    ///
    /// The receipt's bound frontier must equal the independently verified frontier
    /// supplied by the caller. This does not mint a new identity or reservation.
    pub fn restore_pending_execution_with_inventory_at_anchor(
        &self,
        receipt: &ProcessExecutionReceipt,
        expected_anchor: &ExecutionStateAnchor,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
    ) -> Result<(), String> {
        if receipt.state_anchor() != expected_anchor {
            return Err(
                "persisted execution receipt state anchor does not match recovery frontier"
                    .to_string(),
            );
        }

        expected_anchor.verify_state(budget, inventory)?;

        let expected = ProcessExecutionReceipt {
            execution_id: receipt.execution_id.clone(),
            process_id: self.id.clone(),
            input_batch_id: receipt.input_batch_id.clone(),
            waste_stream: self.waste_stream.clone(),
            first_inventory_sequence: receipt.first_inventory_sequence,
            energy_sequence: receipt.energy_sequence,
            energy_node_id: receipt.energy_node_id.clone(),
            state_anchor: receipt.state_anchor.clone(),
            run: receipt.run.clone(),
        };

        if expected != *receipt {
            return Err(
                "persisted execution receipt does not match process definition".to_string(),
            );
        }

        self.validate_run_against_budget(
            &receipt.run,
            budget.available_feed_mass_g,
            budget.available_energy_units,
        )?;

        inventory.reserve_source_batch(
            receipt.execution_id(),
            receipt.input_batch_id(),
            receipt.feed_mass_g(),
        )?;

        if let Err(error) = budget.reserve(receipt) {
            let _ = inventory.release_source_batch(receipt.execution_id());
            return Err(error);
        }

        Ok(())
    }

    /// Authorize an executable process against aggregate budgets and its concrete source batch.
    ///
    /// This convenience path reserves the pending execution and immediately
    /// rehydrates its executable proof. Durable adapters should instead call
    /// `authorize_pending_execution_with_inventory`, persist the pending record,
    /// and only then call `resume_pending_execution`.
    pub fn authorize_execution_with_inventory(
        &self,
        execution_id: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
    ) -> Result<ExecutableProcessExecutionReceipt, String> {
        self.authorize_execution_with_inventory_at_anchor(
            execution_id,
            first_inventory_sequence,
            energy_sequence,
            node_id,
            ExecutionStateAnchor::in_memory(),
            run,
            budget,
            inventory,
        )
    }

    /// Authorize an executable process against a caller-supplied durable frontier.
    pub fn authorize_execution_with_inventory_at_anchor(
        &self,
        execution_id: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        state_anchor: ExecutionStateAnchor,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
    ) -> Result<ExecutableProcessExecutionReceipt, String> {
        let receipt = self.authorize_pending_execution_with_inventory_at_anchor(
            execution_id,
            first_inventory_sequence,
            energy_sequence,
            node_id,
            state_anchor,
            run,
            budget,
            inventory,
        )?;
        resume_pending_execution(receipt.execution_id(), budget, inventory)
    }

    /// Atomically reserve a budget-only process authorization for inspection/provenance.
    ///
    /// This budget-only path does not reserve physical source inventory; receipts created
    /// here are suitable for budgeting/provenance inspection but are not executable through
    /// `commit_process_execution`. Use `authorize_execution_with_inventory` for execution.
    ///
    /// Successful authorization mints a budget-only receipt. It intentionally carries no
    /// concrete physical-source proof and cannot materialize causal events or cross
    /// the executable commit/abort boundary. Reserved capacity is released with
    /// `abort_budget_only_execution` if the inspection authorization is canceled.
    pub fn authorize_execution(
        &self,
        execution_id: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
    ) -> Result<BudgetOnlyProcessExecutionReceipt, String> {
        let execution_id = execution_id.into();
        let node_id = node_id.into();

        if execution_id.is_empty() {
            return Err("execution requires a non-empty execution ID".to_string());
        }

        if node_id.is_empty() {
            return Err("execution requires a non-empty energy node ID".to_string());
        }

        self.validate_run_against_budget(
            &run,
            budget.available_feed_mass_g,
            budget.available_energy_units,
        )?;

        let receipt = ProcessExecutionReceipt {
            execution_id,
            process_id: self.id.clone(),
            input_batch_id: run.input_batch_id.clone(),
            waste_stream: self.waste_stream.clone(),
            first_inventory_sequence,
            energy_sequence,
            energy_node_id: node_id,
            state_anchor: ExecutionStateAnchor::in_memory(),
            run,
        };

        budget.reserve(&receipt)?;
        Ok(BudgetOnlyProcessExecutionReceipt::new(receipt))
    }
}

/// One executed process event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRun {
    pub process_id: String,
    pub input_material: String,
    pub input_batch_id: String,
    pub feed_mass_g: u64,
    pub output_mass_g: BTreeMap<String, u64>,
    pub waste_mass_g: u64,
    pub energy_units: u64,
}

impl ProcessRun {
    #[must_use]
    pub fn new(
        process_id: impl Into<String>,
        input_material: impl Into<String>,
        input_batch_id: impl Into<String>,
        feed_mass_g: u64,
        output_mass_g: BTreeMap<String, u64>,
        waste_mass_g: u64,
        energy_units: u64,
    ) -> Self {
        Self {
            process_id: process_id.into(),
            input_material: input_material.into(),
            input_batch_id: input_batch_id.into(),
            feed_mass_g,
            output_mass_g,
            waste_mass_g,
            energy_units,
        }
    }

    /// Check strict mass conservation for this process execution.
    pub fn validate_mass_balance(&self) -> Result<(), String> {
        let recovered_output = self
            .output_mass_g
            .values()
            .try_fold(0_u64, |sum, mass| sum.checked_add(*mass))
            .ok_or_else(|| "output mass overflow".to_string())?;

        let accounted = recovered_output
            .checked_add(self.waste_mass_g)
            .ok_or_else(|| "accounted mass overflow".to_string())?;

        if accounted == self.feed_mass_g {
            Ok(())
        } else {
            Err(format!(
                "process mass imbalance: feed={} accounted={accounted}",
                self.feed_mass_g
            ))
        }
    }

    /// Return total product mass without permitting integer overflow.
    pub fn total_output_mass_g(&self) -> Result<u64, String> {
        self.output_mass_g
            .values()
            .try_fold(0_u64, |sum, mass| sum.checked_add(*mass))
            .ok_or_else(|| "output mass overflow".to_string())
    }
}

/// Opaque journal/state frontier used to bind durable execution recovery.
///
/// The kernel does not interpret the frontier or depend on a persistence backend.
/// A durable adapter should construct it from a verified append-only journal head
/// and the exact kernel state represented at that frontier, then pass that value
/// unchanged through authorization and recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStateAnchor {
    domain: String,
    frontier: String,
    state_commitment: String,
}

impl ExecutionStateAnchor {
    const IN_MEMORY_DOMAIN: &'static str = "symtropy.execution.in-memory";
    const IN_MEMORY_FRONTIER: &'static str = "UNANCHORED";
    const UNBOUND_STATE_COMMITMENT: &'static str = "UNBOUND";

    /// Construct a bounded, portable execution-state anchor.
    ///
    /// new intentionally leaves the state commitment unbound. It is suitable
    /// for identity-only comparisons, but durable execution APIs require a
    /// state-bound anchor produced by for_state or with_state_commitment.
    pub fn new(
        domain: impl Into<String>,
        frontier: impl Into<String>,
    ) -> Result<Self, String> {
        let domain = domain.into();
        let frontier = frontier.into();

        let portable = |value: &str| {
            !value.is_empty()
                && value.len() <= 256
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'.' | b'-' | b'_' | b':')
                })
        };

        if !portable(&domain) || domain.len() > 128 {
            return Err("execution anchor requires a non-empty portable domain".to_string());
        }
        if !portable(&frontier) {
            return Err("execution anchor requires a non-empty portable frontier".to_string());
        }

        Ok(Self {
            domain,
            frontier,
            state_commitment: Self::UNBOUND_STATE_COMMITMENT.to_string(),
        })
    }

    /// Bind this anchor to the exact pre-Pending kernel state.
    #[must_use]
    pub fn for_state(
        domain: impl Into<String>,
        frontier: impl Into<String>,
        budget: &ExecutionBudget,
        inventory: &InventoryLedger,
    ) -> Result<Self, String> {
        Self::new(domain, frontier)?.with_state_commitment(&execution_state_commitment(
            budget, inventory,
        ))
    }

    /// Attach an externally computed canonical state commitment.
    pub fn with_state_commitment(self, state_commitment: impl Into<String>) -> Result<Self, String> {
        let state_commitment = state_commitment.into();
        if !is_sha256_hex(&state_commitment) {
            return Err("execution anchor requires a lowercase SHA-256 state commitment".to_string());
        }
        if self.domain == Self::IN_MEMORY_DOMAIN && self.frontier == Self::IN_MEMORY_FRONTIER {
            return Err(
                "in-memory execution anchor cannot be promoted to a durable state-bound anchor"
                    .to_string(),
            );
        }

        Ok(Self {
            state_commitment,
            ..self
        })
    }

    /// Explicit anchor for purely in-memory callers.
    ///
    /// This sentinel is intentionally not a durable journal position. Durable
    /// adapters must supply their verified journal/state frontier explicitly.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            domain: Self::IN_MEMORY_DOMAIN.to_string(),
            frontier: Self::IN_MEMORY_FRONTIER.to_string(),
            state_commitment: Self::UNBOUND_STATE_COMMITMENT.to_string(),
        }
    }

    #[must_use]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    #[must_use]
    pub fn frontier(&self) -> &str {
        &self.frontier
    }

    /// Return the canonical pre-Pending state commitment bound to this anchor.
    #[must_use]
    pub fn state_commitment(&self) -> &str {
        &self.state_commitment
    }

    #[must_use]
    pub fn is_state_bound(&self) -> bool {
        self.state_commitment != Self::UNBOUND_STATE_COMMITMENT
    }

    fn verify_state(
        &self,
        budget: &ExecutionBudget,
        inventory: &InventoryLedger,
    ) -> Result<(), String> {
        if !self.is_state_bound() {
            if *self == Self::in_memory() {
                return Ok(());
            }

            return Err(
                "durable execution state anchor is missing its state commitment".to_string(),
            );
        }

        let actual = execution_state_commitment(budget, inventory);
        if actual != self.state_commitment {
            return Err(format!(
                "execution state commitment mismatch: expected={}, actual={}",
                self.state_commitment, actual
            ));
        }

        Ok(())
    }
}

/// Immutable authorization receipt for one funded process execution.
///
/// The receipt owns the process/run identity and the reserved input/energy
/// quantities. Event identities are derived from the execution ID, so re-emitting
/// the same receipt creates duplicate event IDs that the ledgers reject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessExecutionReceipt {
    execution_id: String,
    process_id: String,
    input_batch_id: String,
    waste_stream: String,
    first_inventory_sequence: u64,
    energy_sequence: u64,
    energy_node_id: String,
    state_anchor: ExecutionStateAnchor,
    run: ProcessRun,
}

impl ProcessExecutionReceipt {
    /// Reconstruct a raw receipt from persisted fields without granting executable authority.
    ///
    /// The returned value remains a raw receipt: it cannot materialize causal events or
    /// cross the physical commit/abort boundary. A caller restoring persisted work must
    /// pass it through `ProductionProcess::restore_pending_execution_with_inventory`.
    pub fn from_persisted_parts(
        execution_id: impl Into<String>,
        process_id: impl Into<String>,
        input_batch_id: impl Into<String>,
        waste_stream: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        energy_node_id: impl Into<String>,
        state_anchor: ExecutionStateAnchor,
        run: ProcessRun,
    ) -> Result<Self, String> {
        let execution_id = execution_id.into();
        let process_id = process_id.into();
        let input_batch_id = input_batch_id.into();
        let waste_stream = waste_stream.into();
        let energy_node_id = energy_node_id.into();

        if execution_id.is_empty() {
            return Err("persisted receipt requires a non-empty execution ID".to_string());
        }
        if process_id.is_empty() {
            return Err("persisted receipt requires a non-empty process ID".to_string());
        }
        if input_batch_id.is_empty() {
            return Err("persisted receipt requires a non-empty input batch ID".to_string());
        }
        if waste_stream.is_empty() {
            return Err("persisted receipt requires a non-empty waste stream".to_string());
        }
        if energy_node_id.is_empty() {
            return Err("persisted receipt requires a non-empty energy node ID".to_string());
        }
        if run.process_id != process_id {
            return Err("persisted receipt process ID does not match process run".to_string());
        }
        if run.input_batch_id != input_batch_id {
            return Err("persisted receipt input batch ID does not match process run".to_string());
        }

        Ok(Self {
            execution_id,
            process_id,
            input_batch_id,
            waste_stream,
            first_inventory_sequence,
            energy_sequence,
            energy_node_id,
            state_anchor,
            run,
        })
    }

    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    #[must_use]
    pub fn process_id(&self) -> &str {
        &self.process_id
    }

    #[must_use]
    pub fn input_batch_id(&self) -> &str {
        &self.input_batch_id
    }

    /// Declared waste stream identity carried by this receipt.
    ///
    /// Durable adapters must persist this independently of the generated waste
    /// event so replay never has to infer the stream from event ordering.
    #[must_use]
    pub fn waste_stream(&self) -> &str {
        &self.waste_stream
    }

    #[must_use]
    pub const fn first_inventory_sequence(&self) -> u64 {
        self.first_inventory_sequence
    }

    #[must_use]
    pub const fn energy_sequence(&self) -> u64 {
        self.energy_sequence
    }

    #[must_use]
    pub fn energy_node_id(&self) -> &str {
        &self.energy_node_id
    }

    /// Return the journal/state frontier bound to this execution.
    #[must_use]
    pub const fn state_anchor(&self) -> &ExecutionStateAnchor {
        &self.state_anchor
    }

    /// Return exact causal event identities this receipt would materialize.
    ///
    /// These are metadata only; raw receipts still cannot materialize event payloads.
    #[must_use]
    pub fn inventory_event_ids(&self) -> Vec<String> {
        let mut ids = Vec::with_capacity(self.run.output_mass_g.len() + 2);
        ids.push(format!("{}:inventory:consume", self.execution_id));
        for stream in self.run.output_mass_g.keys() {
            ids.push(format!("{}:inventory:produce:{stream}", self.execution_id));
        }
        ids.push(format!(
            "{}:inventory:waste:{}",
            self.execution_id, self.waste_stream
        ));
        ids
    }

    #[must_use]
    pub fn energy_event_id(&self) -> String {
        format!("{}:energy:consume", self.execution_id)
    }

    /// Return a canonical SHA-256 commitment of this exact receipt.
    ///
    /// Durable adapters should persist this value inside the authenticated
    /// lifecycle record so recovery can detect receipt-field tampering before
    /// invoking any authorization or reservation operation.
    #[must_use]
    pub fn commitment(&self) -> String {
        let mut hasher = CommitmentHasher::new("symtropy.execution.receipt.v1");
        hasher.receipt(self);
        hasher.finish()
    }

    /// Borrow the complete deterministic process run carried by this receipt.
    ///
    /// This is intentionally read-only: recovery layers can serialize the exact
    /// run without gaining a constructor that bypasses authorization.
    #[must_use]
    pub const fn run(&self) -> &ProcessRun {
        &self.run
    }

    #[must_use]
    pub const fn feed_mass_g(&self) -> u64 {
        self.run.feed_mass_g
    }

    #[must_use]
    pub const fn energy_units(&self) -> u64 {
        self.run.energy_units
    }

    /// Materialize the receipt into uniquely identified causal inventory events.
    ///
    /// This method is private to the raw receipt: the public API exposes event
    /// materialization only through ExecutableProcessExecutionReceipt.
    fn inventory_events(&self) -> Result<Vec<InventoryEvent>, String> {
        let cause = format!("execution:{}", self.execution_id);
        let mut events = Vec::with_capacity(self.run.output_mass_g.len() + 2);

        events.push(
            InventoryEvent::new(
                self.first_inventory_sequence,
                self.input_batch_id.clone(),
                self.run.feed_mass_g,
                InventoryEventKind::Consumed,
            )
            .with_event_id(format!("{}:inventory:consume", self.execution_id))
            .with_provenance(cause.clone()),
        );

        for (offset, (stream, mass)) in self.run.output_mass_g.iter().enumerate() {
            let sequence = self
                .first_inventory_sequence
                .checked_add(offset as u64 + 1)
                .ok_or_else(|| "process inventory sequence overflow".to_string())?;
            let batch_id = format!(
                "{}:{}:{}:{stream}",
                self.execution_id, self.process_id, self.first_inventory_sequence
            );

            events.push(
                InventoryEvent::new(sequence, batch_id, *mass, InventoryEventKind::Produced)
                    .with_event_id(format!("{}:inventory:produce:{stream}", self.execution_id))
                    .with_provenance(cause.clone()),
            );
        }

        let waste_sequence = self
            .first_inventory_sequence
            .checked_add(self.run.output_mass_g.len() as u64 + 1)
            .ok_or_else(|| "process inventory sequence overflow".to_string())?;
        let waste_batch_id = format!(
            "{}:{}:{}:{waste}",
            self.execution_id,
            self.process_id,
            self.first_inventory_sequence,
            waste = self.waste_stream
        );
        events.push(
            InventoryEvent::new(
                waste_sequence,
                waste_batch_id,
                self.run.waste_mass_g,
                InventoryEventKind::Produced,
            )
            .with_event_id(format!(
                "{}:inventory:waste:{waste}",
                self.execution_id,
                waste = self.waste_stream
            ))
            .with_provenance(cause),
        );

        Ok(events)
    }

    /// Materialize the receipt into one uniquely identified causal energy event.
    ///
    /// This method is private to the raw receipt: the public API exposes event
    /// materialization only through ExecutableProcessExecutionReceipt.
    #[must_use]
    fn energy_event(&self) -> EnergyEvent {
        EnergyEvent::new(
            self.energy_sequence,
            self.energy_node_id.clone(),
            self.run.energy_units,
            EnergyEventKind::Consumed,
        )
        .with_event_id(format!("{}:energy:consume", self.execution_id))
        .with_provenance(format!("execution:{}", self.execution_id))
    }
}

/// Budget-only authorization receipt.
///
/// This distinct type prevents a caller from using a physically executable
/// receipt with the budget-only cancellation API. It exposes the underlying
/// receipt read-only for inspection but has no causal event materialization
/// methods and cannot cross the physical execution boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetOnlyProcessExecutionReceipt {
    receipt: ProcessExecutionReceipt,
}

impl BudgetOnlyProcessExecutionReceipt {
    fn new(receipt: ProcessExecutionReceipt) -> Self {
        Self { receipt }
    }

    /// Read-only execution identity without granting raw-receipt coercion.
    #[must_use]
    pub fn execution_id(&self) -> &str {
        self.receipt.execution_id()
    }

    /// Read-only process identity without granting raw-receipt coercion.
    #[must_use]
    pub fn process_id(&self) -> &str {
        self.receipt.process_id()
    }

    /// Read-only source-batch identity without granting raw-receipt coercion.
    #[must_use]
    pub fn input_batch_id(&self) -> &str {
        self.receipt.input_batch_id()
    }

    /// Read-only waste-stream identity without granting raw-receipt coercion.
    #[must_use]
    pub fn waste_stream(&self) -> &str {
        self.receipt.waste_stream()
    }

    #[must_use]
    pub const fn first_inventory_sequence(&self) -> u64 {
        self.receipt.first_inventory_sequence()
    }

    #[must_use]
    pub const fn energy_sequence(&self) -> u64 {
        self.receipt.energy_sequence()
    }

    /// Read-only energy-node identity without granting raw-receipt coercion.
    #[must_use]
    pub fn energy_node_id(&self) -> &str {
        self.receipt.energy_node_id()
    }

    #[must_use]
    pub const fn state_anchor(&self) -> &ExecutionStateAnchor {
        self.receipt.state_anchor()
    }

    /// Return the canonical commitment of the receipt carried by this proof.
    #[must_use]
    pub fn commitment(&self) -> String {
        self.receipt.commitment()
    }

    #[must_use]
    pub const fn run(&self) -> &ProcessRun {
        self.receipt.run()
    }

    #[must_use]
    pub const fn feed_mass_g(&self) -> u64 {
        self.receipt.feed_mass_g()
    }

    #[must_use]
    pub const fn energy_units(&self) -> u64 {
        self.receipt.energy_units()
    }
}

/// Execution receipt whose source batch has been reserved for physical execution.
///
/// This type is distinct from ProcessExecutionReceipt: aggregate budget
/// authorization alone cannot produce a value accepted by the commit/abort
/// boundaries. The constructor is private so executable receipts can only be
/// minted after concrete source reservation succeeds. This makes the physical
/// reservation proof part of the Rust type boundary rather than only a runtime
/// convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableProcessExecutionReceipt {
    receipt: ProcessExecutionReceipt,
}

impl ExecutableProcessExecutionReceipt {
    fn new(receipt: ProcessExecutionReceipt) -> Self {
        Self { receipt }
    }

    /// Read-only execution identity without granting raw-receipt coercion.
    #[must_use]
    pub fn execution_id(&self) -> &str {
        self.receipt.execution_id()
    }

    /// Read-only process identity without granting raw-receipt coercion.
    #[must_use]
    pub fn process_id(&self) -> &str {
        self.receipt.process_id()
    }

    /// Read-only source-batch identity without granting raw-receipt coercion.
    #[must_use]
    pub fn input_batch_id(&self) -> &str {
        self.receipt.input_batch_id()
    }

    /// Read-only waste-stream identity without granting raw-receipt coercion.
    #[must_use]
    pub fn waste_stream(&self) -> &str {
        self.receipt.waste_stream()
    }

    #[must_use]
    pub const fn first_inventory_sequence(&self) -> u64 {
        self.receipt.first_inventory_sequence()
    }

    #[must_use]
    pub const fn energy_sequence(&self) -> u64 {
        self.receipt.energy_sequence()
    }

    /// Read-only energy-node identity without granting raw-receipt coercion.
    #[must_use]
    pub fn energy_node_id(&self) -> &str {
        self.receipt.energy_node_id()
    }

    #[must_use]
    pub const fn state_anchor(&self) -> &ExecutionStateAnchor {
        self.receipt.state_anchor()
    }

    /// Return the canonical commitment of the receipt carried by this proof.
    #[must_use]
    pub fn commitment(&self) -> String {
        self.receipt.commitment()
    }

    #[must_use]
    pub const fn run(&self) -> &ProcessRun {
        self.receipt.run()
    }

    #[must_use]
    pub const fn feed_mass_g(&self) -> u64 {
        self.receipt.feed_mass_g()
    }

    #[must_use]
    pub const fn energy_units(&self) -> u64 {
        self.receipt.energy_units()
    }

    /// Materialize causal inventory only after physical source reservation proof.
    pub fn inventory_events(&self) -> Result<Vec<InventoryEvent>, String> {
        self.receipt.inventory_events()
    }

    /// Materialize causal energy only after physical source reservation proof.
    #[must_use]
    pub fn energy_event(&self) -> EnergyEvent {
        self.receipt.energy_event()
    }
}

/// Rehydrate an executable receipt from the still-pending authorization state.
///
/// This does not create new capacity or a new reservation. It verifies that the
/// budget still holds the exact receipt and that the concrete physical source
/// reservation still matches, then returns a fresh typed execution proof.
pub fn resume_pending_execution(
    execution_id: &str,
    budget: &ExecutionBudget,
    inventory: &InventoryLedger,
) -> Result<ExecutableProcessExecutionReceipt, String> {
    let receipt = budget
        .pending_receipt(execution_id)
        .ok_or_else(|| format!("execution is not pending: {execution_id}"))?;

    inventory.matching_source_reservation(
        receipt.execution_id(),
        receipt.input_batch_id(),
        receipt.feed_mass_g(),
    )?;

    Ok(ExecutableProcessExecutionReceipt::new(receipt.clone()))
}

/// Cancel a budget-only execution authorization.
///
/// This releases the reserved feedstock and energy without requiring a
/// concrete inventory reservation. It is the compensation path for the
/// inspection/provenance authorization API, which intentionally does not
/// cross the executable physical-source boundary.
pub fn abort_budget_only_execution(
    receipt: &BudgetOnlyProcessExecutionReceipt,
    budget: &mut ExecutionBudget,
) -> Result<(), String> {
    let inner = &receipt.receipt;
    let reservation = budget
        .reservation(inner.execution_id())
        .ok_or_else(|| format!("execution is not pending: {}", inner.execution_id()))?;

    if reservation != inner {
        return Err("execution reservation does not match receipt".to_string());
    }

    let mut staged_budget = budget.clone();
    staged_budget.abort(inner.execution_id())?;
    *budget = staged_budget;

    Ok(())
}

/// Abort a pending process execution using only its raw authorization receipt.
///
/// This is intentionally safe to call before executable activation, including
/// when a durability adapter fails while recording the pending receipt. It
/// releases the concrete source reservation and restores reserved capacity,
/// but cannot materialize product or energy events.
pub fn abort_pending_execution(
    receipt: &ProcessExecutionReceipt,
    budget: &mut ExecutionBudget,
    inventory: &mut InventoryLedger,
) -> Result<(), String> {
    let reservation = budget
        .reservation(receipt.execution_id())
        .ok_or_else(|| format!("execution is not pending: {}", receipt.execution_id()))?;

    if reservation != receipt {
        return Err("execution reservation does not match receipt".to_string());
    }

    inventory.matching_source_reservation(
        receipt.execution_id(),
        receipt.input_batch_id(),
        receipt.feed_mass_g(),
    )?;

    let mut staged_inventory = inventory.clone();
    staged_inventory.release_source_batch(receipt.execution_id())?;

    let mut staged_budget = budget.clone();
    staged_budget.abort(receipt.execution_id())?;

    *inventory = staged_inventory;
    *budget = staged_budget;

    Ok(())
}

/// Commit one authorized process execution across budget, inventory, and energy.
///
/// Inventory, energy, and budget state are all staged before any live
/// state is replaced. The reservation is settled on a staged budget only after
/// both ledgers succeed, so a failed commit remains retryable without partial
/// state.
pub fn commit_process_execution(
    receipt: &ExecutableProcessExecutionReceipt,
    budget: &mut ExecutionBudget,
    inventory: &mut InventoryLedger,
    energy: &mut EnergyLedger,
) -> Result<(), String> {
    let inner = &receipt.receipt;
    let reservation = budget
        .reservation(inner.execution_id())
        .ok_or_else(|| format!("execution is not pending: {}", inner.execution_id()))?;

    if reservation != inner {
        return Err("execution reservation does not match receipt".to_string());
    }

    let inventory_events = inner.inventory_events()?;
    let energy_event = inner.energy_event();

    let mut staged_inventory = inventory.clone();
    staged_inventory.append_reserved_process_batch(
        receipt.execution_id(),
        receipt.input_batch_id(),
        receipt.feed_mass_g(),
        &inventory_events,
    )?;

    let mut staged_energy = energy.clone();
    staged_energy.append_batch(std::slice::from_ref(&energy_event))?;

    let mut staged_budget = budget.clone();
    staged_budget.settle(receipt.execution_id())?;

    *inventory = staged_inventory;
    *energy = staged_energy;
    *budget = staged_budget;

    Ok(())
}

/// Abort one authorized process execution without mutating either ledger.
///
/// The reserved feedstock and energy are returned to staged budget
/// state, the concrete source reservation is released, and only then are the
/// live states replaced. This keeps abort atomic across authorization state and
/// physical source reservation.
pub fn abort_process_execution(
    receipt: &ExecutableProcessExecutionReceipt,
    budget: &mut ExecutionBudget,
    inventory: &mut InventoryLedger,
) -> Result<(), String> {
    abort_pending_execution(&receipt.receipt, budget, inventory)
}

/// Deterministic process score for comparing candidate bootstrap transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessEfficiency {
    pub capability_gain: u64,
    pub feed_mass_g: u64,
    pub energy_units: u64,
}

impl ProcessEfficiency {
    #[must_use]
    pub const fn new(capability_gain: u64, feed_mass_g: u64, energy_units: u64) -> Self {
        Self {
            capability_gain,
            feed_mass_g,
            energy_units,
        }
    }

    /// Compare capability gained per combined mass-energy burden exactly.
    ///
    /// This is intentionally a structural comparator rather than a physical
    /// economic claim; higher layers decide the actual weighting of resources.
    #[must_use]
    pub fn better_than(self, other: Self) -> bool {
        let self_burden = u128::from(self.feed_mass_g) + u128::from(self.energy_units);
        let other_burden = u128::from(other.feed_mass_g) + u128::from(other.energy_units);

        match (self_burden, other_burden) {
            (0, 0) => self.capability_gain > other.capability_gain,
            (0, _) => self.capability_gain > 0,
            (_, 0) => false,
            _ => ratio_greater(
                u128::from(self.capability_gain),
                self_burden,
                u128::from(other.capability_gain),
                other_burden,
            ),
        }
    }
}

/// A candidate industrial transition represented by independently auditable dimensions.
///
/// The frontier deliberately avoids collapsing capability gain, imported mass,
/// energy, time, and failure risk into one scalar. Higher-is-better dimensions
/// are maximized; burden dimensions are minimized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapCandidate {
    id: String,
    critical_weight_closed_gain: u64,
    dependency_weight_removed: u64,
    imported_mass_g: u64,
    energy_units: u64,
    time_ticks: u64,
    failure_risk_ppm: u64,
}

impl BootstrapCandidate {
    /// Construct a candidate only when every bounded metric is valid.
    pub fn new(
        id: impl Into<String>,
        critical_weight_closed_gain: u64,
        dependency_weight_removed: u64,
        imported_mass_g: u64,
        energy_units: u64,
        time_ticks: u64,
        failure_risk_ppm: u64,
    ) -> Result<Self, String> {
        let id = id.into();
        if id.is_empty() {
            return Err("bootstrap candidate requires a non-empty ID".to_string());
        }
        if failure_risk_ppm > 1_000_000 {
            return Err("bootstrap candidate failure risk must be <= 1,000,000 ppm".to_string());
        }

        Ok(Self {
            id,
            critical_weight_closed_gain,
            dependency_weight_removed,
            imported_mass_g,
            energy_units,
            time_ticks,
            failure_risk_ppm,
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn critical_weight_closed_gain(&self) -> u64 {
        self.critical_weight_closed_gain
    }

    #[must_use]
    pub const fn dependency_weight_removed(&self) -> u64 {
        self.dependency_weight_removed
    }

    #[must_use]
    pub const fn imported_mass_g(&self) -> u64 {
        self.imported_mass_g
    }

    #[must_use]
    pub const fn energy_units(&self) -> u64 {
        self.energy_units
    }

    #[must_use]
    pub const fn time_ticks(&self) -> u64 {
        self.time_ticks
    }

    #[must_use]
    pub const fn failure_risk_ppm(&self) -> u64 {
        self.failure_risk_ppm
    }

    /// Whether this candidate is strictly Pareto-better than another.
    #[must_use]
    pub fn dominates(&self, other: &Self) -> bool {
        let no_worse = self.critical_weight_closed_gain >= other.critical_weight_closed_gain
            && self.dependency_weight_removed >= other.dependency_weight_removed
            && self.imported_mass_g <= other.imported_mass_g
            && self.energy_units <= other.energy_units
            && self.time_ticks <= other.time_ticks
            && self.failure_risk_ppm <= other.failure_risk_ppm;

        let strictly_better = self.critical_weight_closed_gain > other.critical_weight_closed_gain
            || self.dependency_weight_removed > other.dependency_weight_removed
            || self.imported_mass_g < other.imported_mass_g
            || self.energy_units < other.energy_units
            || self.time_ticks < other.time_ticks
            || self.failure_risk_ppm < other.failure_risk_ppm;

        no_worse && strictly_better
    }
}

/// Return the deterministic non-dominated bootstrap candidates.
///
/// Input ordering does not affect the frontier. Candidates with identical IDs
/// are not deduplicated, leaving provenance-preserving callers responsible for
/// deciding whether identically measured actions are distinct opportunities.
#[must_use]
pub fn pareto_frontier(candidates: &[BootstrapCandidate]) -> Vec<BootstrapCandidate> {
    let mut frontier = candidates
        .iter()
        .enumerate()
        .filter(|(index, candidate)| {
            !candidates
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != *index && other.dominates(candidate))
        })
        .map(|(_, candidate)| candidate.clone())
        .collect::<Vec<_>>();

    frontier.sort_by(|left, right| left.id.cmp(&right.id));
    frontier
}

/// Exact rational representation of capability gain per imported mass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapEfficiency {
    pub capability_gain: u64,
    pub imported_mass_g: u64,
}

impl BootstrapEfficiency {
    #[must_use]
    pub const fn new(capability_gain: u64, imported_mass_g: u64) -> Self {
        Self {
            capability_gain,
            imported_mass_g,
        }
    }

    /// Compare efficiencies without floating-point rounding.
    #[must_use]
    pub fn better_than(self, other: Self) -> bool {
        match (self.imported_mass_g, other.imported_mass_g) {
            (0, 0) => self.capability_gain > other.capability_gain,
            (0, _) => self.capability_gain > 0,
            (_, 0) => false,
            _ => {
                (self.capability_gain as u128) * (other.imported_mass_g as u128)
                    > (other.capability_gain as u128) * (self.imported_mass_g as u128)
            }
        }
    }
}

/// Explicit recovery outcome for a failed dependency or facility class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Immediate,
    RecoveredAfter { ticks: u64 },
    Unrecoverable,
}

/// Convert recovery outcome into a deterministic horizon.
#[must_use]
pub const fn recovery_horizon(outcome: &RecoveryOutcome) -> Option<u64> {
    match outcome {
        RecoveryOutcome::Immediate => Some(0),
        RecoveryOutcome::RecoveredAfter { ticks } => Some(*ticks),
        RecoveryOutcome::Unrecoverable => None,
    }
}

/// Energy ledger event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyEventKind {
    Generated,
    Consumed,
    Recovered,
}

/// One append-only energy event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyEvent {
    pub sequence: u64,
    pub event_id: Option<String>,
    pub node_id: String,
    pub energy_units: u64,
    pub kind: EnergyEventKind,
    pub provenance_id: Option<String>,
}

impl EnergyEvent {
    #[must_use]
    pub fn new(
        sequence: u64,
        node_id: impl Into<String>,
        energy_units: u64,
        kind: EnergyEventKind,
    ) -> Self {
        Self {
            sequence,
            event_id: None,
            node_id: node_id.into(),
            energy_units,
            kind,
            provenance_id: None,
        }
    }

    /// Attach a unique identity to an energy event.
    #[must_use]
    pub fn with_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = Some(event_id.into());
        self
    }

    /// Attach a stable causal provenance identifier to an energy event.
    #[must_use]
    pub fn with_provenance(mut self, provenance_id: impl Into<String>) -> Self {
        self.provenance_id = Some(provenance_id.into());
        self
    }
}

/// Stateful append-only energy ledger.
///
/// The live append boundary enforces unique event identities, a strictly
/// increasing sequence frontier, and deterministic balance conservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyLedger {
    state: BTreeMap<String, u64>,
    events: Vec<EnergyEvent>,
    seen_event_ids: BTreeSet<String>,
    last_sequence: Option<u64>,
}

impl EnergyLedger {
    #[must_use]
    pub fn new(initial: BTreeMap<String, u64>) -> Self {
        Self {
            state: initial,
            events: Vec::new(),
            seen_event_ids: BTreeSet::new(),
            last_sequence: None,
        }
    }

    /// Append one event atomically.
    pub fn append(&mut self, event: EnergyEvent) -> Result<(), String> {
        self.append_batch(std::slice::from_ref(&event))
    }

    /// Validate and commit a complete event batch atomically.
    pub fn append_batch(&mut self, events: &[EnergyEvent]) -> Result<(), String> {
        let mut staged_balances = BTreeMap::new();
        let mut staged_event_ids: BTreeSet<String> = BTreeSet::new();
        let mut next_sequence = self.last_sequence;

        for event in events {
            let event_id = event
                .event_id
                .as_deref()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| "energy event requires non-empty event ID".to_string())?;

            if event.provenance_id.as_deref().is_none_or(str::is_empty) {
                return Err("energy event requires non-empty provenance".to_string());
            }

            if event.node_id.is_empty() {
                return Err("energy event requires non-empty node ID".to_string());
            }

            if self.seen_event_ids.contains(event_id)
                || !staged_event_ids.insert(event_id.to_string())
            {
                return Err("duplicate energy event ID".to_string());
            }

            if let Some(last_sequence) = next_sequence {
                if event.sequence <= last_sequence {
                    return Err("energy event sequence must increase".to_string());
                }
            }

            let node_id = event.node_id.clone();
            let balance = staged_balances
                .get(&node_id)
                .copied()
                .or_else(|| self.state.get(&node_id).copied())
                .unwrap_or(0);

            let next_balance = match event.kind {
                EnergyEventKind::Generated | EnergyEventKind::Recovered => balance
                    .checked_add(event.energy_units)
                    .ok_or_else(|| "energy overflow".to_string())?,
                EnergyEventKind::Consumed => {
                    if balance < event.energy_units {
                        return Err("energy underflow".to_string());
                    }
                    balance - event.energy_units
                }
            };

            staged_balances.insert(node_id, next_balance);
            next_sequence = Some(event.sequence);
        }

        for (node_id, balance) in staged_balances {
            self.state.insert(node_id, balance);
        }
        self.seen_event_ids.extend(staged_event_ids);
        self.last_sequence = next_sequence;
        self.events.extend(events.iter().cloned());
        Ok(())
    }

    #[must_use]
    pub fn state(&self) -> &BTreeMap<String, u64> {
        &self.state
    }

    #[must_use]
    pub fn events(&self) -> &[EnergyEvent] {
        &self.events
    }

    /// Return a deterministic SHA-256 commitment of the complete energy state.
    #[must_use]
    pub fn state_commitment(&self) -> String {
        let mut hasher = CommitmentHasher::new("symtropy.execution.energy.v1");
        hasher.map_u64(&self.state);
        hasher.u64(self.events.len() as u64);
        for event in &self.events {
            hasher.energy_event(event);
        }
        hasher.string_set(&self.seen_event_ids);
        hasher.optional_u64(self.last_sequence);
        hasher.finish()
    }
}

/// Replay energy from an initial balance and append-only causal events.
pub fn replay_energy(
    initial: &BTreeMap<String, u64>,
    events: &[EnergyEvent],
) -> Result<BTreeMap<String, u64>, String> {
    let mut ordered = events.to_vec();
    ordered.sort_by_key(|event| event.sequence);

    if ordered
        .windows(2)
        .any(|window| window[0].sequence == window[1].sequence)
    {
        return Err("duplicate energy event sequence".to_string());
    }

    let mut ledger = EnergyLedger::new(initial.clone());
    for event in ordered {
        ledger.append(event)?;
    }

    Ok(ledger.state().clone())
}

/// Verify final energy against deterministic replay of its causal history.
pub fn verify_energy_conservation(
    initial: &BTreeMap<String, u64>,
    events: &[EnergyEvent],
    observed_final: &BTreeMap<String, u64>,
) -> Result<(), String> {
    if replay_energy(initial, events)? == *observed_final {
        Ok(())
    } else {
        Err("observed energy differs from causal replay".to_string())
    }
}

/// Confidence/evidence grade attached to a resource claim.
///
/// Claims become inventory-eligible only through an explicit certification
/// transition. This prevents an estimated deposit from silently becoming stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceGrade {
    Modelled,
    RemoteObserved,
    InSituMeasured,
    ProcessDemonstrated,
}

/// A bounded resource claim that is not yet inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceClaim {
    id: String,
    mass_g: u64,
    evidence: EvidenceGrade,
    confidence_ppm: u64,
}

impl ResourceClaim {
    /// Construct a resource claim only when identity and bounded confidence are valid.
    pub fn new(
        id: impl Into<String>,
        mass_g: u64,
        evidence: EvidenceGrade,
        confidence_ppm: u64,
    ) -> Result<Self, String> {
        let id = id.into();
        if id.is_empty() {
            return Err("resource claim requires a non-empty ID".to_string());
        }
        if mass_g == 0 {
            return Err("resource claim must have non-zero mass".to_string());
        }
        if confidence_ppm > 1_000_000 {
            return Err("resource claim confidence must be <= 1,000,000 ppm".to_string());
        }

        Ok(Self {
            id,
            mass_g,
            evidence,
            confidence_ppm,
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn mass_g(&self) -> u64 {
        self.mass_g
    }

    #[must_use]
    pub const fn evidence(&self) -> EvidenceGrade {
        self.evidence
    }

    #[must_use]
    pub const fn confidence_ppm(&self) -> u64 {
        self.confidence_ppm
    }

    /// Promote a claim into an inventory-eligible certificate.
    pub fn certify_for_inventory(
        &self,
        minimum_evidence: EvidenceGrade,
        minimum_confidence_ppm: u64,
    ) -> Result<CertifiedResource, String> {
        if self.mass_g == 0 {
            return Err("resource claim must have non-zero mass".to_string());
        }
        if self.confidence_ppm > 1_000_000 {
            return Err("resource claim confidence is out of range".to_string());
        }
        if minimum_confidence_ppm > 1_000_000 {
            return Err("minimum confidence must be <= 1,000,000 ppm".to_string());
        }

        if self.evidence < minimum_evidence {
            return Err(format!(
                "insufficient evidence grade: required={minimum_evidence:?}, observed={:?}",
                self.evidence
            ));
        }

        if self.confidence_ppm < minimum_confidence_ppm {
            return Err(format!(
                "insufficient confidence: required={minimum_confidence_ppm}, observed={}",
                self.confidence_ppm
            ));
        }

        if self.id.is_empty() {
            return Err("resource claim requires a non-empty ID".to_string());
        }

        Ok(CertifiedResource {
            certificate_id: format!("resource-certificate:{}", self.id),
            claim_id: self.id.clone(),
            mass_g: self.mass_g,
        })
    }
}

/// An explicitly certified resource quantity that may enter inventory.
///
/// Fields are private so callers cannot construct inventory-authorizing
/// certificates without passing through the evidence gate above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertifiedResource {
    certificate_id: String,
    claim_id: String,
    mass_g: u64,
}

impl CertifiedResource {
    #[must_use]
    pub fn certificate_id(&self) -> &str {
        &self.certificate_id
    }

    #[must_use]
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    #[must_use]
    pub const fn mass_g(&self) -> u64 {
        self.mass_g
    }
}

/// Causal inventory event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryEventKind {
    Produced,
    Consumed,
}

/// One append-only inventory event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEvent {
    pub sequence: u64,
    pub event_id: Option<String>,
    pub batch_id: String,
    pub mass_g: u64,
    pub kind: InventoryEventKind,
    pub provenance_id: Option<String>,
}

impl InventoryEvent {
    #[must_use]
    pub fn new(
        sequence: u64,
        batch_id: impl Into<String>,
        mass_g: u64,
        kind: InventoryEventKind,
    ) -> Self {
        Self {
            sequence,
            event_id: None,
            batch_id: batch_id.into(),
            mass_g,
            kind,
            provenance_id: None,
        }
    }

    /// Attach a unique identity to an inventory event.
    #[must_use]
    pub fn with_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = Some(event_id.into());
        self
    }

    /// Attach a stable causal provenance identifier to an event.
    #[must_use]
    pub fn with_provenance(mut self, provenance_id: impl Into<String>) -> Self {
        self.provenance_id = Some(provenance_id.into());
        self
    }

    /// Create a produced inventory event from an explicitly certified resource.
    #[must_use]
    pub fn from_certified_resource(
        sequence: u64,
        batch_id: impl Into<String>,
        resource: &CertifiedResource,
    ) -> Self {
        Self::new(
            sequence,
            batch_id,
            resource.mass_g,
            InventoryEventKind::Produced,
        )
        .with_event_id(resource.certificate_id().to_string())
        .with_provenance(resource.claim_id().to_string())
    }
}

/// Stateful append-only inventory ledger.
///
/// Unlike replay, this models the live append boundary. Accepted events advance
/// both the sequence frontier and unique event-ID set. Failed appends leave the
/// ledger unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryLedger {
    state: BTreeMap<String, u64>,
    events: Vec<InventoryEvent>,
    seen_event_ids: BTreeSet<String>,
    source_reservations: BTreeMap<String, InventorySourceReservation>,
    last_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InventorySourceReservation {
    execution_id: String,
    batch_id: String,
    mass_g: u64,
}

impl InventoryLedger {
    #[must_use]
    pub fn new(initial: BTreeMap<String, u64>) -> Self {
        Self {
            state: initial,
            events: Vec::new(),
            seen_event_ids: BTreeSet::new(),
            source_reservations: BTreeMap::new(),
            last_sequence: None,
        }
    }

    /// Reserve a concrete source batch for one pending executable authorization.
    ///
    /// Reservations are physical-stock claims separate from aggregate budget
    /// authorization. A reserved batch cannot be consumed by another append
    /// until the owning execution commits or releases it. The method is private:
    /// callers cannot create an orphaned physical lock outside the authorization
    /// lifecycle.
    fn reserve_source_batch(
        &mut self,
        execution_id: impl Into<String>,
        batch_id: impl Into<String>,
        mass_g: u64,
    ) -> Result<(), String> {
        let execution_id = execution_id.into();
        let batch_id = batch_id.into();

        if execution_id.is_empty() {
            return Err("source reservation requires a non-empty execution ID".to_string());
        }
        if batch_id.is_empty() {
            return Err("source reservation requires a non-empty batch ID".to_string());
        }
        if mass_g == 0 {
            return Err("source reservation requires non-zero mass".to_string());
        }
        if self.source_reservations.contains_key(&execution_id) {
            return Err(format!("source batch already reserved: {execution_id}"));
        }

        let already_reserved = self
            .source_reservations
            .values()
            .filter(|reservation| reservation.batch_id == batch_id)
            .try_fold(0_u64, |sum, reservation| {
                sum.checked_add(reservation.mass_g)
            })
            .ok_or_else(|| "source reservation mass overflow".to_string())?;

        let available = self.state.get(&batch_id).copied().unwrap_or(0);
        let total_reserved = already_reserved
            .checked_add(mass_g)
            .ok_or_else(|| "source reservation mass overflow".to_string())?;

        if total_reserved > available {
            return Err(format!(
                "insufficient unreserved source batch: batch={batch_id}, required={mass_g}, available={}",
                available.saturating_sub(already_reserved)
            ));
        }

        self.source_reservations.insert(
            execution_id.clone(),
            InventorySourceReservation {
                execution_id,
                batch_id,
                mass_g,
            },
        );

        Ok(())
    }

    fn release_source_batch(&mut self, execution_id: &str) -> Result<(), String> {
        self.source_reservations
            .remove(execution_id)
            .map(|_| ())
            .ok_or_else(|| format!("source batch reservation is not pending: {execution_id}"))
    }

    fn matching_source_reservation(
        &self,
        execution_id: &str,
        batch_id: &str,
        mass_g: u64,
    ) -> Result<(), String> {
        let reservation = self
            .source_reservations
            .get(execution_id)
            .ok_or_else(|| format!("source batch reservation is not pending: {execution_id}"))?;

        if reservation.execution_id != execution_id
            || reservation.batch_id != batch_id
            || reservation.mass_g != mass_g
        {
            return Err("source batch reservation does not match execution".to_string());
        }

        Ok(())
    }

    fn append_reserved_process_batch(
        &mut self,
        execution_id: &str,
        batch_id: &str,
        mass_g: u64,
        events: &[InventoryEvent],
    ) -> Result<(), String> {
        self.matching_source_reservation(execution_id, batch_id, mass_g)?;

        let expected_provenance = format!("execution:{execution_id}");
        let matching_consumes = events
            .iter()
            .filter(|event| {
                event.kind == InventoryEventKind::Consumed && event.batch_id == batch_id
            })
            .collect::<Vec<_>>();

        if matching_consumes.len() != 1
            || matching_consumes[0].mass_g != mass_g
            || matching_consumes[0].provenance_id.as_deref() != Some(expected_provenance.as_str())
        {
            return Err(
                "reserved source batch is not consumed exactly by its execution".to_string(),
            );
        }

        let mut staged = self.clone();
        staged.release_source_batch(execution_id)?;

        let preserved = staged
            .source_reservations
            .iter()
            .filter(|(_, reservation)| reservation.batch_id == batch_id)
            .map(|(id, reservation)| (id.clone(), reservation.clone()))
            .collect::<Vec<_>>();

        for (id, _) in &preserved {
            staged.source_reservations.remove(id);
        }

        if let Err(error) = staged.append_batch(events) {
            return Err(error);
        }

        for (id, reservation) in preserved {
            staged.source_reservations.insert(id, reservation);
        }

        *self = staged;

        Ok(())
    }

    /// Append one event atomically.
    pub fn append(&mut self, event: InventoryEvent) -> Result<(), String> {
        self.append_batch(std::slice::from_ref(&event))
    }

    /// Validate and commit a complete event batch atomically.
    ///
    /// All identities, sequence numbers, provenance, and balance changes are
    /// staged first. No state or history is mutated until the entire batch passes.
    pub fn append_batch(&mut self, events: &[InventoryEvent]) -> Result<(), String> {
        let mut staged_balances = BTreeMap::new();
        let mut staged_event_ids: BTreeSet<String> = BTreeSet::new();
        let mut next_sequence = self.last_sequence;

        for event in events {
            let event_id = event
                .event_id
                .as_deref()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| "inventory event requires non-empty event ID".to_string())?;

            if event.provenance_id.as_deref().is_none_or(str::is_empty) {
                return Err("inventory event requires non-empty provenance".to_string());
            }

            if event.batch_id.is_empty() {
                return Err("inventory event requires non-empty batch ID".to_string());
            }

            if self.seen_event_ids.contains(event_id)
                || !staged_event_ids.insert(event_id.to_string())
            {
                return Err("duplicate inventory event ID".to_string());
            }

            if let Some(last_sequence) = next_sequence {
                if event.sequence <= last_sequence {
                    return Err("inventory event sequence must increase".to_string());
                }
            }

            if event.kind == InventoryEventKind::Consumed
                && self
                    .source_reservations
                    .values()
                    .any(|reservation| reservation.batch_id == event.batch_id)
            {
                return Err(format!(
                    "inventory source batch is reserved: {}",
                    event.batch_id
                ));
            }

            let batch_id = event.batch_id.clone();
            let balance = staged_balances
                .get(&batch_id)
                .copied()
                .or_else(|| self.state.get(&batch_id).copied())
                .unwrap_or(0);

            let next_balance = match event.kind {
                InventoryEventKind::Produced => balance
                    .checked_add(event.mass_g)
                    .ok_or_else(|| "inventory overflow".to_string())?,
                InventoryEventKind::Consumed => {
                    if balance < event.mass_g {
                        return Err("inventory underflow".to_string());
                    }
                    balance - event.mass_g
                }
            };

            staged_balances.insert(batch_id, next_balance);
            next_sequence = Some(event.sequence);
        }

        for (batch_id, balance) in staged_balances {
            self.state.insert(batch_id, balance);
        }
        self.seen_event_ids.extend(staged_event_ids);
        self.last_sequence = next_sequence;
        self.events.extend(events.iter().cloned());
        Ok(())
    }

    #[must_use]
    pub fn state(&self) -> &BTreeMap<String, u64> {
        &self.state
    }

    #[must_use]
    pub fn events(&self) -> &[InventoryEvent] {
        &self.events
    }

    /// Return a deterministic SHA-256 commitment of the complete inventory state.
    #[must_use]
    pub fn state_commitment(&self) -> String {
        let mut hasher = CommitmentHasher::new("symtropy.execution.inventory.v1");
        hasher.map_u64(&self.state);

        hasher.u64(self.events.len() as u64);
        for event in &self.events {
            hasher.inventory_event(event);
        }

        hasher.string_set(&self.seen_event_ids);

        hasher.u64(self.source_reservations.len() as u64);
        for (execution_id, reservation) in &self.source_reservations {
            hasher.string(execution_id);
            hasher.string(&reservation.execution_id);
            hasher.string(&reservation.batch_id);
            hasher.u64(reservation.mass_g);
        }

        hasher.optional_u64(self.last_sequence);
        hasher.finish()
    }
}


/// Return the canonical commitment of the exact budget/inventory state represented by an anchor.
#[must_use]
pub fn execution_state_commitment(
    budget: &ExecutionBudget,
    inventory: &InventoryLedger,
) -> String {
    combine_execution_state_commitments(
        budget.state_commitment().as_str(),
        inventory.state_commitment().as_str(),
    )
}

/// Combine independently canonical budget and inventory commitments into the pre-Pending state commitment.
#[must_use]
pub fn combine_execution_state_commitments(
    budget_commitment: &str,
    inventory_commitment: &str,
) -> String {
    let mut hasher = CommitmentHasher::new("symtropy.execution.pre-pending-state.v1");
    hasher.u64(1);
    hasher.bytes(budget_commitment.as_bytes());
    hasher.bytes(inventory_commitment.as_bytes());
    hasher.finish()
}

/// Compare two positive rational numbers without cross-product overflow.
fn ratio_greater(mut left_num: u128, mut left_den: u128, mut right_num: u128, mut right_den: u128) -> bool {
    let mut reverse = false;

    loop {
        let left_quotient = left_num / left_den;
        let right_quotient = right_num / right_den;
        if left_quotient != right_quotient {
            return if reverse {
                left_quotient < right_quotient
            } else {
                left_quotient > right_quotient
            };
        }

        let left_remainder = left_num % left_den;
        let right_remainder = right_num % right_den;

        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return false,
            (true, false) => return reverse,
            (false, true) => return !reverse,
            (false, false) => {
                left_num = left_den;
                left_den = left_remainder;
                right_num = right_den;
                right_den = right_remainder;
                reverse = !reverse;
            }
        }
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

struct CommitmentHasher {
    hasher: Sha256,
}

impl CommitmentHasher {
    fn new(domain: &str) -> Self {
        let mut hasher = Sha256::new();
        Self::string_into(&mut hasher, domain);
        Self { hasher }
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.hasher.update((bytes.len() as u64).to_le_bytes());
        self.hasher.update(bytes);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.hasher.update(value.to_le_bytes());
    }

    fn optional_u64(&mut self, value: Option<u64>) {
        match value {
            Some(value) => {
                self.byte(1);
                self.u64(value);
            }
            None => self.byte(0),
        }
    }

    fn byte(&mut self, value: u8) {
        self.hasher.update([value]);
    }

    fn string_set(&mut self, values: &BTreeSet<String>) {
        self.u64(values.len() as u64);
        for value in values {
            self.string(value);
        }
    }

    fn map_u64(&mut self, values: &BTreeMap<String, u64>) {
        self.u64(values.len() as u64);
        for (key, value) in values {
            self.string(key);
            self.u64(*value);
        }
    }

    fn receipt_map(&mut self, values: &BTreeMap<String, ProcessExecutionReceipt>) {
        self.u64(values.len() as u64);
        for (execution_id, receipt) in values {
            self.string(execution_id);
            self.receipt(receipt);
        }
    }

    fn receipt(&mut self, receipt: &ProcessExecutionReceipt) {
        self.string(receipt.execution_id());
        self.string(receipt.process_id());
        self.string(receipt.input_batch_id());
        self.string(receipt.waste_stream());
        self.u64(receipt.first_inventory_sequence());
        self.u64(receipt.energy_sequence());
        self.string(receipt.energy_node_id());
        self.string(receipt.state_anchor().domain());
        self.string(receipt.state_anchor().frontier());
        self.string(receipt.state_anchor().state_commitment());
        self.process_run(receipt.run());
    }

    fn process_run(&mut self, run: &ProcessRun) {
        self.string(&run.process_id);
        self.string(&run.input_material);
        self.string(&run.input_batch_id);
        self.u64(run.feed_mass_g);
        self.map_u64(&run.output_mass_g);
        self.u64(run.waste_mass_g);
        self.u64(run.energy_units);
    }

    fn inventory_event(&mut self, event: &InventoryEvent) {
        self.u64(event.sequence);
        match &event.event_id {
            Some(event_id) => {
                self.byte(1);
                self.string(event_id);
            }
            None => self.byte(0),
        }
        self.string(&event.batch_id);
        self.u64(event.mass_g);
        self.byte(match event.kind {
            InventoryEventKind::Produced => 0,
            InventoryEventKind::Consumed => 1,
        });
        match &event.provenance_id {
            Some(provenance_id) => {
                self.byte(1);
                self.string(provenance_id);
            }
            None => self.byte(0),
        }
    }

    fn energy_event(&mut self, event: &EnergyEvent) {
        self.u64(event.sequence);
        match &event.event_id {
            Some(event_id) => {
                self.byte(1);
                self.string(event_id);
            }
            None => self.byte(0),
        }
        self.string(&event.node_id);
        self.u64(event.energy_units);
        self.byte(match event.kind {
            EnergyEventKind::Generated => 0,
            EnergyEventKind::Consumed => 1,
            EnergyEventKind::Recovered => 2,
        });
        match &event.provenance_id {
            Some(provenance_id) => {
                self.byte(1);
                self.string(provenance_id);
            }
            None => self.byte(0),
        }
    }

    fn finish(self) -> String {
        let digest = self.hasher.finalize();
        hex_digest(&digest)
    }

    fn string_into(hasher: &mut Sha256, value: &str) {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

/// Replay inventory from an initial balance and append-only causal events.
///
/// Events are sorted by sequence so input ordering cannot change the result.
/// Duplicate sequence numbers, duplicate event identities, and underflow are
/// rejected. Recycling is represented as an explicit source consumption followed
/// by destination production; it is never an implicit mass creation operation.
pub fn replay_inventory(
    initial: &BTreeMap<String, u64>,
    events: &[InventoryEvent],
) -> Result<BTreeMap<String, u64>, String> {
    let mut ordered = events.to_vec();
    ordered.sort_by_key(|event| event.sequence);

    if ordered
        .windows(2)
        .any(|window| window[0].sequence == window[1].sequence)
    {
        return Err("duplicate inventory event sequence".to_string());
    }

    let mut ledger = InventoryLedger::new(initial.clone());
    for event in ordered {
        ledger.append(event)?;
    }

    Ok(ledger.state().clone())
}

/// Verify final inventory against deterministic replay of its causal history.
pub fn verify_inventory_conservation(
    initial: &BTreeMap<String, u64>,
    events: &[InventoryEvent],
    observed_final: &BTreeMap<String, u64>,
) -> Result<(), String> {
    if replay_inventory(initial, events)? == *observed_final {
        Ok(())
    } else {
        Err("observed inventory differs from causal replay".to_string())
    }
}

/// Deterministic ratio in integer parts-per-million.
#[must_use]
pub const fn ratio_ppm(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        0
    } else {
        let value = ((numerator as u128) * 1_000_000u128) / (denominator as u128);
        if value > 1_000_000 {
            1_000_000
        } else {
            value as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bottleneck_graph() -> DependencyGraph {
        DependencyGraph::new(
            [
                Capability::new("mine", 30, ["rock"]),
                Capability::new("refine", 40, ["mine", "chem"]),
                Capability::new("controller", 30, ["electronics"]),
            ],
            [
                Dependency::new("rock", DependencyClass::LocalClosed),
                Dependency::new("chem", DependencyClass::LocalClosed),
                Dependency::new("electronics", DependencyClass::ImportedDurable),
            ],
        )
    }

    #[test]
    fn tiny_imported_bottleneck_is_visible() {
        let graph = bottleneck_graph();
        let report = graph.evaluate(9_000, 1_000);

        assert_eq!(report.mass_closure_ppm, 900_000);
        assert_eq!(report.critical_closure_ppm, 700_000);
        assert!(!report.fully_closed());
    }

    #[test]
    fn excessive_dependency_depth_fails_closed() {
        let mut capabilities = Vec::with_capacity(MAX_DEPENDENCY_RESOLUTION_DEPTH + 1);
        for index in 0..=MAX_DEPENDENCY_RESOLUTION_DEPTH {
            let dependencies = if index == 0 {
                Vec::new()
            } else {
                vec![format!("capability-{previous}", previous = index - 1)]
            };
            capabilities.push(Capability::new(
                format!("capability-{index}"),
                1,
                dependencies,
            ));
        }

        let graph = DependencyGraph::new(capabilities, std::iter::empty::<Dependency>());
        let report = graph.evaluate(1, 0);

        assert!(!report.valid);
        assert!(
            report
                .definition_errors()
                .iter()
                .any(|error| error == "dependency resolution depth exceeded: 1024")
        );
        assert!(
            report.assessments.iter().any(|assessment| {
                assessment
                    .unresolved_dependencies
                    .iter()
                    .any(|dependency| dependency == "resolution-depth-exceeded:1024")
            })
        );
    }

    #[test]
    fn unknown_dependency_fails_closed() {
        let graph = DependencyGraph::new(
            [Capability::new("rover", 100, ["mystery_part"])],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(1, 0);

        assert!(!report.assessments[0].closed);
        assert_eq!(
            report.assessments[0].unresolved_dependencies,
            vec!["mystery_part".to_string()]
        );
    }

    #[test]
    fn zero_critical_weight_cannot_claim_full_closure() {
        let graph = DependencyGraph::new(
            [Capability::new("noncritical", 0, ["steel"])],
            [Dependency::new("steel", DependencyClass::LocalClosed)],
        );
        let report = graph.evaluate(100, 0);

        assert!(report.valid);
        assert_eq!(report.weighted_critical_total, 0);
        assert_eq!(report.critical_closure_ppm, 0);
        assert!(!report.fully_closed());
    }

    #[test]
    fn mass_closure_overflow_fails_closed() {
        let graph = DependencyGraph::new(
            [Capability::new("seed", 1, std::iter::empty::<String>())],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(u64::MAX, 1);

        assert!(!report.valid);
        assert_eq!(report.mass_closure_ppm, 0);
        assert!(
            report
                .definition_errors()
                .iter()
                .any(|error| error == "mass total overflow")
        );
    }

    #[test]
    fn critical_weight_sum_overflow_fails_closed() {
        let graph = DependencyGraph::new(
            [
                Capability::new("a", u64::MAX, std::iter::empty::<String>()),
                Capability::new("b", 1, std::iter::empty::<String>()),
            ],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(100, 0);

        assert!(!report.valid);
        assert!(!report.fully_closed());
        assert_eq!(report.critical_closure_ppm, 0);
        assert!(
            report
                .definition_errors()
                .iter()
                .any(|error| error == "critical weight sum overflow")
        );
        assert!(
            report
                .assessments
                .iter()
                .all(|assessment| !assessment.closed)
        );
    }

    #[test]
    fn empty_stage_requirement_cannot_qualify_a_stage() {
        let graph = DependencyGraph::new(
            [Capability::new("seed", 100, std::iter::empty::<String>())],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(100, 0);

        let requirements = [StageRequirement::new(
            ClosureStage::Seed,
            std::iter::empty::<String>(),
        )];
        assert_eq!(highest_closed_stage(&report, &requirements), None);
    }

    #[test]
    fn duplicate_stage_requirements_fail_closed() {
        let graph = DependencyGraph::new(
            [Capability::new("seed", 100, std::iter::empty::<String>())],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(100, 0);

        let requirements = [
            StageRequirement::new(ClosureStage::Seed, ["seed"]),
            StageRequirement::new(ClosureStage::Seed, ["seed", "missing"]),
        ];
        assert_eq!(highest_closed_stage(&report, &requirements), None);
    }

    #[test]
    fn recursive_capability_chain_closes() {
        let graph = DependencyGraph::new(
            [
                Capability::new("motor", 50, ["steel"]),
                Capability::new("rover", 50, ["motor"]),
            ],
            [Dependency::new("steel", DependencyClass::LocalClosed)],
        );
        let report = graph.evaluate(1, 0);

        assert!(report.fully_closed());
        assert!(
            report
                .assessments
                .iter()
                .all(|assessment| assessment.closed)
        );
    }

    #[test]
    fn cycles_fail_closed() {
        let graph = DependencyGraph::new(
            [
                Capability::new("a", 50, ["b"]),
                Capability::new("b", 50, ["a"]),
            ],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(1, 0);

        assert!(
            report
                .assessments
                .iter()
                .any(|assessment| assessment.cycle_detected)
        );
        assert!(!report.fully_closed());
    }

    #[test]
    fn failed_dependency_provenance_survives_downstream_cache() {
        let graph = DependencyGraph::new(
            [
                Capability::new("seed", 30, ["missing"]),
                Capability::new("downstream", 70, ["seed"]),
            ],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(1, 0);

        let downstream = report
            .assessments
            .iter()
            .find(|assessment| assessment.id == "downstream")
            .expect("downstream capability exists");

        assert_eq!(downstream.unresolved_dependencies, vec!["missing"]);
    }

    #[test]
    fn duplicate_graph_definitions_fail_closed() {
        let graph = DependencyGraph::new(
            [
                Capability::new("rover", 50, ["steel"]),
                Capability::new("rover", 50, ["electronics"]),
            ],
            [
                Dependency::new("steel", DependencyClass::LocalClosed),
                Dependency::new("steel", DependencyClass::ImportedDurable),
            ],
        );
        let report = graph.evaluate(1_000, 0);

        assert!(!report.valid);
        assert!(!report.fully_closed());
        assert!(
            report.assessments[0]
                .unresolved_dependencies
                .iter()
                .any(|dependency| dependency == "definition:duplicate capability ID: rover")
        );
        assert!(
            report.assessments[0]
                .unresolved_dependencies
                .iter()
                .any(|dependency| dependency == "definition:duplicate dependency ID: steel")
        );
        assert_eq!(report.critical_closure_ppm, 0);
        assert_eq!(report.mass_closure_ppm, 0);
    }

    #[test]
    fn capability_dependency_namespace_collision_fails_closed() {
        let graph = DependencyGraph::new(
            [Capability::new("steel", 100, std::iter::empty::<String>())],
            [Dependency::new("steel", DependencyClass::LocalClosed)],
        );
        let report = graph.evaluate(100, 0);

        assert!(!report.valid);
        assert!(!report.fully_closed());
        assert_eq!(
            report.definition_errors(),
            &["capability/dependency ID collision: steel".to_string()]
        );
    }

    #[test]
    fn empty_graph_definition_ids_fail_closed() {
        let graph = DependencyGraph::new(
            [Capability::new("", 10, std::iter::empty::<String>())],
            [Dependency::new("", DependencyClass::LocalClosed)],
        );
        let report = graph.evaluate(100, 0);

        assert!(!report.valid);
        assert!(!report.fully_closed());
        assert!(report.assessments.is_empty());
    }

    #[test]
    fn blocker_ranking_exposes_strongest_constraint() {
        let graph = bottleneck_graph();
        let report = graph.evaluate(9_000, 1_000);
        let blockers = graph.rank_blockers(&report).expect("blocker weights fit");

        assert_eq!(blockers[0].id, "electronics");
        assert_eq!(blockers[0].weight, 30);
        assert_eq!(blockers[0].affected_capabilities, vec!["controller"]);
    }

    #[test]
    fn blocker_ranking_rejects_invalid_report() {
        let graph = bottleneck_graph();
        let report = graph.evaluate(u64::MAX, 1);

        let error = graph
            .rank_blockers(&report)
            .expect_err("invalid closure reports must not produce ranked blockers");
        assert_eq!(error, "cannot rank blockers for invalid closure report");
    }

    #[test]
    fn blocker_ranking_overflow_fails_closed() {
        let graph = DependencyGraph::new(
            [
                Capability::new("first", u64::MAX, ["missing"]),
                Capability::new("second", 1, ["missing"]),
            ],
            std::iter::empty::<Dependency>(),
        );
        let report = graph.evaluate(1, 0);

        let error = graph
            .rank_blockers(&report)
            .expect_err("blocker weight overflow must be rejected");
        assert_eq!(error, "blocker weight overflow for dependency: missing");
    }

    #[test]
    fn substitution_counts_as_closed() {
        let graph = DependencyGraph::new(
            [Capability::new("pump", 100, ["seal"])],
            [Dependency::new(
                "seal",
                DependencyClass::ReplaceableBySubstitution,
            )],
        );
        assert!(graph.evaluate(100, 0).fully_closed());
    }

    #[test]
    fn highest_stage_requires_all_declared_capabilities() {
        let graph = DependencyGraph::new(
            [
                Capability::new("repair", 10, ["tooling"]),
                Capability::new("structure", 10, ["repair"]),
            ],
            [Dependency::new("tooling", DependencyClass::LocalClosed)],
        );
        let report = graph.evaluate(1_000, 0);

        let requirements = [
            StageRequirement::new(ClosureStage::Repair, ["repair"]),
            StageRequirement::new(ClosureStage::Structural, ["repair", "structure"]),
            StageRequirement::new(ClosureStage::Expansion, ["missing"]),
        ];

        assert_eq!(
            highest_closed_stage(&report, &requirements),
            Some(ClosureStage::Structural)
        );
    }

    #[test]
    fn stage_progression_cannot_skip_an_unmet_intermediate_stage() {
        let graph = DependencyGraph::new(
            [
                Capability::new("seed", 10, ["steel"]),
                Capability::new("expansion", 10, ["seed"]),
                Capability::new("factory", 10, ["missing"]),
            ],
            [Dependency::new("steel", DependencyClass::LocalClosed)],
        );
        let report = graph.evaluate(1_000, 0);

        let requirements = [
            StageRequirement::new(ClosureStage::Expansion, ["expansion"]),
            StageRequirement::new(ClosureStage::Factory, ["factory"]),
            StageRequirement::new(ClosureStage::Seed, ["seed"]),
        ];

        assert_eq!(
            highest_closed_stage(&report, &requirements),
            Some(ClosureStage::Seed)
        );
    }

    #[test]
    fn empty_process_and_ledger_identifiers_fail_closed() {
        let process = ProductionProcess::new("process", "ore", ["metal"], "");
        let run = ProcessRun::new(
            "process",
            "ore",
            "feed",
            10,
            BTreeMap::from([("metal".to_string(), 10)]),
            0,
            1,
        );
        assert!(process.validate_run(&run).is_err());

        let mut inventory = InventoryLedger::new(BTreeMap::new());
        let inventory_event = InventoryEvent::new(1, "", 1, InventoryEventKind::Produced)
            .with_event_id("inventory-empty-batch")
            .with_provenance("test");
        assert!(inventory.append(inventory_event).is_err());
        assert!(inventory.events().is_empty());

        let mut energy = EnergyLedger::new(BTreeMap::new());
        let energy_event = EnergyEvent::new(1, "", 1, EnergyEventKind::Generated)
            .with_event_id("energy-empty-node")
            .with_provenance("test");
        assert!(energy.append(energy_event).is_err());
        assert!(energy.events().is_empty());
    }

    #[test]
    fn process_co_products_must_balance_mass() {
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-001",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        run.validate_mass_balance()
            .expect("co-product process should conserve mass");
        assert_eq!(
            run.total_output_mass_g().expect("output mass should fit"),
            900
        );
    }

    #[test]
    fn total_output_mass_fails_closed_on_overflow() {
        let run = ProcessRun::new(
            "overflow",
            "feed",
            "batch-overflow",
            u64::MAX,
            BTreeMap::from([("a".to_string(), u64::MAX), ("b".to_string(), 1)]),
            0,
            0,
        );

        assert!(run.total_output_mass_g().is_err());
        assert!(run.validate_mass_balance().is_err());
    }

    #[test]
    fn invalid_closure_report_cannot_qualify_a_stage() {
        let report = ClosureReport {
            assessments: vec![CapabilityAssessment {
                id: "seed".to_string(),
                closed: true,
                unresolved_dependencies: Vec::new(),
                cycle_detected: false,
            }],
            weighted_critical_closed: 1,
            weighted_critical_total: 1,
            critical_closure_ppm: 1_000_000,
            mass_closure_ppm: 1_000_000,
            valid: false,
            definition_errors: vec!["invalid".to_string()],
        };

        let requirements = [StageRequirement::new(ClosureStage::Seed, ["seed"])];
        assert_eq!(highest_closed_stage(&report, &requirements), None);
    }

    #[test]
    fn process_definition_rejects_ambiguous_stream_schema() {
        let duplicate = ProductionProcess::new(
            "duplicate-streams",
            "regolith",
            ["oxygen", "oxygen"],
            "waste",
        );
        let run = ProcessRun::new(
            "duplicate-streams",
            "regolith",
            "feed-schema",
            10,
            BTreeMap::from([("oxygen".to_string(), 10)]),
            0,
            1,
        );
        assert!(duplicate.validate_run(&run).is_err());

        let collision =
            ProductionProcess::new("waste-collision", "regolith", ["oxygen", "waste"], "waste");
        let collision_run = ProcessRun::new(
            "waste-collision",
            "regolith",
            "feed-collision",
            10,
            BTreeMap::from([("oxygen".to_string(), 10)]),
            0,
            1,
        );
        assert!(collision.validate_run(&collision_run).is_err());
    }

    #[test]
    fn process_run_must_match_declared_schema() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let valid = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-001",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        process
            .validate_run(&valid)
            .expect("declared process run should validate");

        let undeclared = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-001",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("gold".to_string(), 720)]),
            100,
            4_000,
        );
        assert!(process.validate_run(&undeclared).is_err());

        let wrong_process = ProcessRun::new(
            "unrelated_process",
            "regolith",
            "feed-001",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );
        assert!(process.validate_run(&wrong_process).is_err());
    }

    #[test]
    fn process_execution_requires_atomic_budget_authorization() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-006",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(2_000, 8_000);
        let receipt = process
            .authorize_execution("exec-006", 40, 41, "bus", run.clone(), &mut budget)
            .expect("funded process should authorize");

        assert_eq!(budget.available_feed_mass_g(), 1_000);
        assert_eq!(budget.available_energy_units(), 4_000);
        assert_eq!(receipt.execution_id(), "exec-006");
        assert_eq!(receipt.feed_mass_g(), 1_000);
        assert_eq!(receipt.energy_units(), 4_000);

        assert!(
            process
                .authorize_execution("exec-006", 50, 51, "bus", run.clone(), &mut budget)
                .is_err()
        );
        assert!(
            process
                .authorize_execution("exec-007", 50, 51, "bus", run, &mut budget)
                .is_ok()
        );
        assert_eq!(budget.available_feed_mass_g(), 0);
        assert_eq!(budget.available_energy_units(), 0);
    }

    #[test]
    fn source_batch_reservation_prevents_competing_executions() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run_a = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "shared-feed",
            700,
            BTreeMap::from([("oxygen".to_string(), 126), ("metal".to_string(), 504)]),
            70,
            2_800,
        );
        let run_b = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "shared-feed",
            400,
            BTreeMap::from([("oxygen".to_string(), 72), ("metal".to_string(), 288)]),
            40,
            1_600,
        );

        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("shared-feed".to_string(), 1_000)]));
        let mut budget = ExecutionBudget::new(2_000, 8_000);

        process
            .authorize_execution_with_inventory(
                "exec-source-a",
                100,
                200,
                "bus",
                run_a,
                &mut budget,
                &mut inventory,
            )
            .expect("first source reservation should succeed");

        assert!(
            process
                .authorize_execution_with_inventory(
                    "exec-source-b",
                    101,
                    201,
                    "bus",
                    run_b,
                    &mut budget,
                    &mut inventory,
                )
                .is_err()
        );

        assert_eq!(inventory.state().get("shared-feed"), Some(&1_000));
        assert!(inventory.events().is_empty());
        assert_eq!(budget.available_feed_mass_g(), 1_300);
        assert_eq!(budget.available_energy_units(), 5_200);
    }

    #[test]
    fn partial_source_reservations_can_settle_independently() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run_a = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "shared-feed-independent",
            600,
            BTreeMap::from([("oxygen".to_string(), 108), ("metal".to_string(), 432)]),
            60,
            2_400,
        );
        let run_b = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "shared-feed-independent",
            400,
            BTreeMap::from([("oxygen".to_string(), 72), ("metal".to_string(), 288)]),
            40,
            1_600,
        );

        let mut inventory = InventoryLedger::new(BTreeMap::from([(
            "shared-feed-independent".to_string(),
            1_000,
        )]));
        let mut budget = ExecutionBudget::new(1_000, 4_000);

        let receipt_a = process
            .authorize_execution_with_inventory(
                "exec-source-a",
                100,
                200,
                "bus",
                run_a,
                &mut budget,
                &mut inventory,
            )
            .expect("first partial reservation should succeed");
        let receipt_b = process
            .authorize_execution_with_inventory(
                "exec-source-b",
                104,
                204,
                "bus",
                run_b,
                &mut budget,
                &mut inventory,
            )
            .expect("second partial reservation should fit remaining stock");

        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 8_000)]));
        commit_process_execution(&receipt_a, &mut budget, &mut inventory, &mut energy)
            .expect("first reservation should settle");

        assert_eq!(inventory.state().get("shared-feed-independent"), Some(&400));
        assert_eq!(inventory.events().len(), 4);

        commit_process_execution(&receipt_b, &mut budget, &mut inventory, &mut energy)
            .expect("remaining reservation should settle");
        assert_eq!(inventory.state().get("shared-feed-independent"), Some(&0));
        assert_eq!(inventory.events().len(), 8);
        assert_eq!(energy.events().len(), 2);
    }

    #[test]
    fn failed_budget_authorization_releases_source_reservation() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let seed = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "seed-feed",
            500,
            BTreeMap::from([("oxygen".to_string(), 90), ("metal".to_string(), 360)]),
            50,
            500,
        );
        let retry = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "release-feed",
            500,
            BTreeMap::from([("oxygen".to_string(), 90), ("metal".to_string(), 360)]),
            50,
            500,
        );

        let mut inventory = InventoryLedger::new(BTreeMap::from([
            ("seed-feed".to_string(), 500),
            ("release-feed".to_string(), 500),
        ]));
        let mut budget = ExecutionBudget::new(1_000, 1_000);

        process
            .authorize_execution("exec-release", 1, 2, "bus", seed, &mut budget)
            .expect("seed execution should occupy the duplicate authorization identity");

        assert!(
            process
                .authorize_execution_with_inventory(
                    "exec-release",
                    3,
                    4,
                    "bus",
                    retry.clone(),
                    &mut budget,
                    &mut inventory,
                )
                .is_err()
        );

        assert_eq!(budget.available_feed_mass_g(), 500);
        assert_eq!(budget.available_energy_units(), 500);
        assert_eq!(inventory.state().get("release-feed"), Some(&500));
        assert!(inventory.events().is_empty());

        process
            .authorize_execution_with_inventory(
                "exec-retry",
                3,
                4,
                "bus",
                retry,
                &mut budget,
                &mut inventory,
            )
            .expect("released source reservation should allow retry");
    }

    #[test]
    fn reserved_source_blocks_unrelated_inventory_consumption() {
        let mut inventory = InventoryLedger::new(BTreeMap::from([("ore-batch".to_string(), 500)]));
        inventory
            .reserve_source_batch("exec-reserved", "ore-batch", 300)
            .expect("source reservation should succeed");

        let unrelated = InventoryEvent::new(1, "ore-batch", 250, InventoryEventKind::Consumed)
            .with_event_id("unrelated-consume")
            .with_provenance("other-execution");

        assert!(inventory.append(unrelated).is_err());
        assert_eq!(inventory.state().get("ore-batch"), Some(&500));
        assert!(inventory.events().is_empty());
    }

    #[test]
    fn budget_only_authorization_does_not_reserve_physical_source() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "unreserved-feed",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let receipt = process
            .authorize_execution("exec-unreserved", 1, 2, "bus", run, &mut budget)
            .expect("budget-only authorization should still be constructible");
        let inventory =
            InventoryLedger::new(BTreeMap::from([("unreserved-feed".to_string(), 1_000)]));

        assert!(budget.reservation(receipt.execution_id()).is_some());
        assert!(inventory.source_reservations.is_empty());
    }

    #[test]
    fn unbudgeted_execution_cannot_mint_a_receipt() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-007",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );
        let mut budget = ExecutionBudget::new(999, 4_000);

        assert!(
            process
                .authorize_execution("exec-008", 60, 61, "bus", run, &mut budget)
                .is_err()
        );
        assert_eq!(budget.available_feed_mass_g(), 999);
        assert_eq!(budget.available_energy_units(), 4_000);
    }

    #[test]
    fn process_execution_receipt_produces_causal_inventory_and_energy_events() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-004",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory = InventoryLedger::new(BTreeMap::from([("feed-004".to_string(), 1_000)]));
        let receipt = process
            .authorize_execution_with_inventory(
                "exec-004",
                10,
                20,
                "power-bus-1",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("valid process should authorize");

        let events = receipt
            .inventory_events()
            .expect("receipt should materialize inventory");
        let energy = receipt.energy_event();

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].kind, InventoryEventKind::Consumed);
        assert_eq!(events[0].batch_id, "feed-004");
        assert_eq!(
            events[0].event_id.as_deref(),
            Some("exec-004:inventory:consume")
        );
        assert_eq!(
            events[0].provenance_id.as_deref(),
            Some("execution:exec-004")
        );
        assert_eq!(events[1].sequence, 11);
        assert_eq!(events[2].sequence, 12);
        assert_eq!(events[3].sequence, 13);
        assert!(events[1].batch_id.ends_with(":oxygen"));
        assert!(events[2].batch_id.ends_with(":metal"));
        assert!(events[3].batch_id.ends_with(":waste"));
        assert_eq!(events[3].mass_g, 100);
        assert_eq!(
            events[3].event_id.as_deref(),
            Some("exec-004:inventory:waste:waste")
        );
        assert_eq!(
            events[3].provenance_id.as_deref(),
            Some("execution:exec-004")
        );
        assert_eq!(energy.event_id.as_deref(), Some("exec-004:energy:consume"));
        assert_eq!(energy.provenance_id.as_deref(), Some("execution:exec-004"));

        let mut inventory = InventoryLedger::new(BTreeMap::from([("feed-004".to_string(), 1_000)]));
        inventory
            .append_batch(&events)
            .expect("receipt should append atomically");
        assert_eq!(inventory.state().get("feed-004"), Some(&0));
        assert_eq!(
            inventory
                .state()
                .get("exec-004:regolith_electrolysis:10:oxygen"),
            Some(&180)
        );
        assert_eq!(
            inventory
                .state()
                .get("exec-004:regolith_electrolysis:10:metal"),
            Some(&720)
        );
        let material_after: u64 = inventory
            .state()
            .iter()
            .filter(|(batch_id, _)| {
                batch_id.as_str() == "exec-004:regolith_electrolysis:10:oxygen"
                    || batch_id.as_str() == "exec-004:regolith_electrolysis:10:metal"
                    || batch_id.as_str() == "exec-004:regolith_electrolysis:10:waste"
            })
            .map(|(_, mass)| *mass)
            .sum();
        assert_eq!(material_after, 1_000);

        let mut energy_ledger =
            EnergyLedger::new(BTreeMap::from([("power-bus-1".to_string(), 5_000)]));
        energy_ledger
            .append_batch(&[energy])
            .expect("energy receipt should append atomically");
        assert_eq!(energy_ledger.state().get("power-bus-1"), Some(&1_000));
    }

    #[test]
    fn process_output_batches_bind_to_execution_identity() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run_a = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-a",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );
        let run_b = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-b",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(2_000, 8_000);
        let mut inventory = InventoryLedger::new(BTreeMap::from([
            ("feed-a".to_string(), 1_000),
            ("feed-b".to_string(), 1_000),
        ]));
        let receipt_a = process
            .authorize_execution_with_inventory(
                "exec-a",
                100,
                200,
                "bus",
                run_a,
                &mut budget,
                &mut inventory,
            )
            .expect("first execution should authorize");
        let receipt_b = process
            .authorize_execution_with_inventory(
                "exec-b",
                100,
                200,
                "bus",
                run_b,
                &mut budget,
                &mut inventory,
            )
            .expect("second execution should authorize");

        let events_a = receipt_a
            .inventory_events()
            .expect("first receipt should materialize inventory");
        let events_b = receipt_b
            .inventory_events()
            .expect("second receipt should materialize inventory");

        assert_eq!(
            events_a[1].batch_id,
            "exec-a:regolith_electrolysis:100:oxygen"
        );
        assert_eq!(
            events_b[1].batch_id,
            "exec-b:regolith_electrolysis:100:oxygen"
        );
        assert_ne!(events_a[1].batch_id, events_b[1].batch_id);
        assert_eq!(
            events_a[3].batch_id,
            "exec-a:regolith_electrolysis:100:waste"
        );
        assert_eq!(
            events_b[3].batch_id,
            "exec-b:regolith_electrolysis:100:waste"
        );
    }

    #[test]
    fn process_execution_commits_budget_inventory_and_energy_as_one_unit() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-transaction",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-transaction".to_string(), 1_000)]));
        let receipt = process
            .authorize_execution_with_inventory(
                "exec-transaction",
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("execution should authorize");
        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5_000)]));

        commit_process_execution(&receipt, &mut budget, &mut inventory, &mut energy)
            .expect("authorized execution should commit atomically");

        assert_eq!(budget.available_feed_mass_g(), 0);
        assert_eq!(budget.available_energy_units(), 0);
        assert_eq!(inventory.state().get("feed-transaction"), Some(&0));
        assert_eq!(inventory.events().len(), 4);
        assert_eq!(
            inventory
                .state()
                .get("exec-transaction:regolith_electrolysis:10:waste"),
            Some(&100)
        );
        assert_eq!(energy.state().get("bus"), Some(&1_000));
        assert_eq!(energy.events().len(), 1);

        assert!(
            commit_process_execution(&receipt, &mut budget, &mut inventory, &mut energy).is_err()
        );
        assert_eq!(inventory.events().len(), 4);
        assert_eq!(energy.events().len(), 1);
    }

    #[test]
    fn execution_reservation_binds_the_complete_receipt() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-binding",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-binding".to_string(), 1_000)]));
        let receipt = process
            .authorize_execution_with_inventory(
                "exec-binding",
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("execution should authorize");

        let mut altered = receipt.clone();
        altered.receipt.first_inventory_sequence = 11;
        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5_000)]));

        assert!(
            commit_process_execution(&altered, &mut budget, &mut inventory, &mut energy).is_err()
        );
        assert!(inventory.events().is_empty());
        assert!(energy.events().is_empty());
        assert_eq!(budget.available_feed_mass_g(), 0);
        assert_eq!(budget.available_energy_units(), 0);

        commit_process_execution(&receipt, &mut budget, &mut inventory, &mut energy)
            .expect("original bound receipt should remain commit-ready");
    }

    #[test]
    fn committed_and_aborted_execution_states_remain_distinguishable() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-terminal",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut commit_budget = ExecutionBudget::new(1_000, 4_000);
        let mut commit_inventory =
            InventoryLedger::new(BTreeMap::from([("feed-terminal".to_string(), 1_000)]));
        let commit_receipt = process
            .authorize_execution_with_inventory(
                "exec-terminal-commit",
                10,
                20,
                "bus",
                run.clone(),
                &mut commit_budget,
                &mut commit_inventory,
            )
            .expect("commit execution should authorize");
        let mut commit_energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5_000)]));
        assert_eq!(
            commit_budget.execution_state("exec-terminal-commit"),
            Some(ExecutionState::Pending)
        );
        let pending_record = commit_budget
            .execution_record("exec-terminal-commit")
            .expect("pending lifecycle record should be available");
        assert_eq!(pending_record.execution_id(), "exec-terminal-commit");
        assert_eq!(pending_record.state(), ExecutionState::Pending);
        assert_eq!(pending_record.receipt(), &commit_receipt.receipt);
        commit_process_execution(
            &commit_receipt,
            &mut commit_budget,
            &mut commit_inventory,
            &mut commit_energy,
        )
        .expect("commit execution should settle");
        assert_eq!(
            commit_budget.execution_state("exec-terminal-commit"),
            Some(ExecutionState::Committed)
        );
        let committed_record = commit_budget
            .execution_record("exec-terminal-commit")
            .expect("committed lifecycle record should remain available");
        assert_eq!(committed_record.execution_id(), "exec-terminal-commit");
        assert_eq!(committed_record.state(), ExecutionState::Committed);
        assert_eq!(committed_record.receipt(), &commit_receipt.receipt);
        assert!(
            commit_budget
                .pending_receipt("exec-terminal-commit")
                .is_none()
        );
        assert!(
            resume_pending_execution("exec-terminal-commit", &commit_budget, &commit_inventory,)
                .is_err()
        );

        let mut abort_budget = ExecutionBudget::new(1_000, 4_000);
        let mut abort_inventory =
            InventoryLedger::new(BTreeMap::from([("feed-terminal".to_string(), 1_000)]));
        let abort_receipt = process
            .authorize_execution_with_inventory(
                "exec-terminal-abort",
                30,
                40,
                "bus",
                run,
                &mut abort_budget,
                &mut abort_inventory,
            )
            .expect("abort execution should authorize");
        abort_process_execution(&abort_receipt, &mut abort_budget, &mut abort_inventory)
            .expect("abort execution should settle");
        assert_eq!(
            abort_budget.execution_state("exec-terminal-abort"),
            Some(ExecutionState::Aborted)
        );
        let aborted_record = abort_budget
            .execution_record("exec-terminal-abort")
            .expect("aborted lifecycle record should remain available");
        assert_eq!(aborted_record.execution_id(), "exec-terminal-abort");
        assert_eq!(aborted_record.state(), ExecutionState::Aborted);
        assert_eq!(aborted_record.receipt(), &abort_receipt.receipt);
        assert!(
            abort_budget
                .pending_receipt("exec-terminal-abort")
                .is_none()
        );
        assert!(
            resume_pending_execution("exec-terminal-abort", &abort_budget, &abort_inventory,)
                .is_err()
        );
    }

    #[test]
    fn budget_only_authorization_can_be_canceled_without_source_reservation() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-budget-only-abort",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let receipt = process
            .authorize_execution("exec-budget-only-abort", 10, 20, "bus", run, &mut budget)
            .expect("budget-only authorization should reserve capacity");

        assert_eq!(budget.available_feed_mass_g(), 0);
        assert_eq!(budget.available_energy_units(), 0);
        abort_budget_only_execution(&receipt, &mut budget)
            .expect("budget-only authorization should cancel safely");
        assert_eq!(budget.available_feed_mass_g(), 1_000);
        assert_eq!(budget.available_energy_units(), 4_000);
        assert_eq!(
            budget.execution_state("exec-budget-only-abort"),
            Some(ExecutionState::Aborted)
        );
        assert!(budget.pending_receipt("exec-budget-only-abort").is_none());
    }

    #[test]
    fn budget_only_receipts_cannot_use_physical_abort_boundary() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-budget-only-boundary",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory = InventoryLedger::new(BTreeMap::new());
        let receipt = process
            .authorize_execution("exec-budget-only-boundary", 10, 20, "bus", run, &mut budget)
            .expect("budget-only authorization should succeed");

        assert_eq!(
            budget.execution_state(receipt.execution_id()),
            Some(ExecutionState::Pending)
        );
        assert!(inventory.source_reservations.is_empty());
        abort_budget_only_execution(&receipt, &mut budget)
            .expect("budget-only cancellation should remain the safe path");
    }

    #[test]
    fn raw_pending_receipt_can_abort_without_executable_activation() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-abort-pending",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-abort-pending".to_string(), 1_000)]));
        let pending = process
            .authorize_pending_execution_with_inventory(
                "exec-abort-pending",
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("pending execution should authorize");

        abort_pending_execution(&pending, &mut budget, &mut inventory)
            .expect("raw pending execution should abort safely");
        assert_eq!(budget.available_feed_mass_g(), 1_000);
        assert_eq!(budget.available_energy_units(), 4_000);
        assert_eq!(
            budget.execution_state("exec-abort-pending"),
            Some(ExecutionState::Aborted)
        );
        assert!(inventory.source_reservations.is_empty());
        assert!(resume_pending_execution("exec-abort-pending", &budget, &inventory).is_err());
    }

    #[test]
    fn execution_state_transitions_are_fail_closed() {
        assert!(ExecutionState::Pending.can_transition_to(ExecutionState::Committed));
        assert!(ExecutionState::Pending.can_transition_to(ExecutionState::Aborted));
        assert!(!ExecutionState::Pending.can_transition_to(ExecutionState::Pending));
        assert!(!ExecutionState::Committed.can_transition_to(ExecutionState::Pending));
        assert!(!ExecutionState::Committed.can_transition_to(ExecutionState::Committed));
        assert!(!ExecutionState::Committed.can_transition_to(ExecutionState::Aborted));
        assert!(!ExecutionState::Aborted.can_transition_to(ExecutionState::Pending));
        assert!(!ExecutionState::Aborted.can_transition_to(ExecutionState::Committed));
        assert!(!ExecutionState::Aborted.can_transition_to(ExecutionState::Aborted));
    }

    #[test]
    fn persisted_receipt_reconstruction_rejects_embedded_identity_mismatch() {
        let process_run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "run-batch",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let process_mismatch = ProcessExecutionReceipt::from_persisted_parts(
            "exec-persisted-mismatch-process",
            "different-process",
            "run-batch",
            "waste",
            10,
            20,
            "bus",
            ExecutionStateAnchor::in_memory(),
            process_run.clone(),
        )
        .expect_err("receipt/process identity mismatch must fail closed");
        assert_eq!(
            process_mismatch,
            "persisted receipt process ID does not match process run"
        );

        let batch_mismatch = ProcessExecutionReceipt::from_persisted_parts(
            "exec-persisted-mismatch-batch",
            "regolith_electrolysis",
            "receipt-batch",
            "waste",
            10,
            20,
            "bus",
            ExecutionStateAnchor::in_memory(),
            process_run,
        )
        .expect_err("receipt/batch identity mismatch must fail closed");
        assert_eq!(
            batch_mismatch,
            "persisted receipt input batch ID does not match process run"
        );
    }

    #[test]
    fn execution_state_anchor_rejects_non_portable_values() {
        assert!(ExecutionStateAnchor::new("", "head").is_err());
        assert!(ExecutionStateAnchor::new("journal", "").is_err());
        assert!(ExecutionStateAnchor::new("journal name", "head").is_err());
        assert!(ExecutionStateAnchor::new("journal", "head value").is_err());
        assert!(ExecutionStateAnchor::new("journal", "head\\nvalue").is_err());
        assert!(ExecutionStateAnchor::new("journal", "head/1").is_err());
        assert!(ExecutionStateAnchor::new("journal:v1", "abcdef0123456789").is_ok());
        assert!(
            ExecutionStateAnchor::new("journal:v1", "head")
                .unwrap()
                .with_state_commitment("not-a-sha256")
                .is_err()
        );
        assert!(
            ExecutionStateAnchor::new("journal:v1", "head")
                .unwrap()
                .with_state_commitment("a".repeat(64))
                .is_ok()
        );
        assert!(
            ExecutionStateAnchor::new("journal:v1", "head")
                .unwrap()
                .with_state_commitment("A".repeat(64))
                .is_err()
        );
    }

    #[test]
    fn execution_state_anchor_rejects_promoted_in_memory_sentinel() {
        assert!(
            ExecutionStateAnchor::in_memory()
                .with_state_commitment("a".repeat(64))
                .is_err()
        );
    }

    #[test]
    fn execution_state_anchor_binds_exact_kernel_state() {
        let budget = ExecutionBudget::new(1_000, 4_000);
        let inventory = InventoryLedger::new(BTreeMap::from([("feed".to_string(), 1_000)]));

        let anchor = ExecutionStateAnchor::for_state(
            "symtropy.execution.journal.v1",
            "head-a",
            &budget,
            &inventory,
        )
        .expect("state-bound anchor should construct");
        assert!(anchor.is_state_bound());
        assert_eq!(anchor.state_commitment().len(), 64);

        let changed_inventory =
            InventoryLedger::new(BTreeMap::from([("feed".to_string(), 999)]));
        let changed_anchor = ExecutionStateAnchor::for_state(
            "symtropy.execution.journal.v1",
            "head-a",
            &budget,
            &changed_inventory,
        )
        .expect("changed state should still produce a commitment");
        assert_ne!(anchor.state_commitment(), changed_anchor.state_commitment());
    }

    #[test]
    fn persisted_pending_restore_rejects_mismatched_state_anchor() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-anchor",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-anchor".to_string(), 1_000)]));
        let creation_anchor = ExecutionStateAnchor::for_state(
            "symtropy.execution.journal.v1",
            "head-a",
            &budget,
            &inventory,
        )
        .expect("creation anchor should bind pre-pending state");

        let receipt = process
            .authorize_pending_execution_with_inventory_at_anchor(
                "exec-anchor",
                10,
                20,
                "bus",
                creation_anchor.clone(),
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("anchored pending execution should authorize");

        let mut recovered_budget = ExecutionBudget::new(1_000, 4_000);
        let mut recovered_inventory =
            InventoryLedger::new(BTreeMap::from([("feed-anchor".to_string(), 1_000)]));
        let later_anchor = ExecutionStateAnchor::for_state(
            "symtropy.execution.journal.v1",
            "head-b",
            &recovered_budget,
            &recovered_inventory,
        )
        .expect("later frontier anchor should bind recovered state");

        assert!(process
            .restore_pending_execution_with_inventory_at_anchor(
                &receipt,
                &later_anchor,
                &mut recovered_budget,
                &mut recovered_inventory,
            )
            .is_err());
        assert_eq!(recovered_budget.available_feed_mass_g(), 1_000);
        assert_eq!(recovered_budget.available_energy_units(), 4_000);
        assert!(recovered_inventory.source_reservations.is_empty());

        process
            .restore_pending_execution_with_inventory_at_anchor(
                &receipt,
                &creation_anchor,
                &mut recovered_budget,
                &mut recovered_inventory,
            )
            .expect("matching state frontier and state should restore");
        assert_eq!(recovered_budget.available_feed_mass_g(), 0);
        assert_eq!(recovered_budget.available_energy_units(), 0);
        assert_eq!(receipt.state_anchor().frontier(), "head-a");
    }

    #[test]
    fn persisted_pending_restore_rejects_same_frontier_with_mismatched_state() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-state-mismatch",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-state-mismatch".to_string(), 1_000)]));
        let anchor = ExecutionStateAnchor::for_state(
            "symtropy.execution.journal.v1",
            "head-state",
            &budget,
            &inventory,
        )
        .expect("state-bound anchor should construct");

        let receipt = process
            .authorize_pending_execution_with_inventory_at_anchor(
                "exec-state-mismatch",
                30,
                40,
                "bus",
                anchor.clone(),
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("pending execution should authorize");

        let mut recovered_budget = ExecutionBudget::new(2_000, 4_000);
        let mut recovered_inventory =
            InventoryLedger::new(BTreeMap::from([("feed-state-mismatch".to_string(), 1_000)]));

        let error = process
            .restore_pending_execution_with_inventory_at_anchor(
                &receipt,
                &anchor,
                &mut recovered_budget,
                &mut recovered_inventory,
            )
            .expect_err("same frontier with different state must fail closed");
        assert!(error.contains("state commitment mismatch"));
        assert_eq!(recovered_budget.available_feed_mass_g(), 2_000);
        assert_eq!(recovered_budget.available_energy_units(), 4_000);
        assert!(recovered_inventory.source_reservations.is_empty());
    }

    #[test]
    fn receipt_commitment_is_deterministic_and_context_bound() {
        let anchor = ExecutionStateAnchor::new(
            "symtropy.execution.journal.v1",
            "head-receipt",
        )
        .expect("valid anchor")
        .with_state_commitment("a".repeat(64))
        .expect("valid commitment");

        let first = ProcessExecutionReceipt::from_persisted_parts(
            "exec-receipt",
            "regolith_electrolysis",
            "feed-receipt",
            "waste",
            10,
            20,
            "bus",
            anchor.clone(),
            ProcessRun::new(
                "regolith_electrolysis",
                "regolith",
                "feed-receipt",
                1_000,
                BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
                100,
                4_000,
            ),
        )
        .expect("first receipt should reconstruct");

        let second = ProcessExecutionReceipt::from_persisted_parts(
            "exec-receipt",
            "regolith_electrolysis",
            "feed-receipt",
            "waste",
            10,
            20,
            "bus",
            anchor,
            first.run().clone(),
        )
        .expect("second receipt should reconstruct");

        assert_eq!(first.commitment(), second.commitment());
        assert!(is_sha256_hex(&first.commitment()));

        let changed_anchor = ExecutionStateAnchor::new(
            "symtropy.execution.journal.v1",
            "different-head",
        )
        .expect("valid anchor")
        .with_state_commitment("a".repeat(64))
        .expect("valid commitment");
        let changed = ProcessExecutionReceipt::from_persisted_parts(
            "exec-receipt",
            "regolith_electrolysis",
            "feed-receipt",
            "waste",
            10,
            20,
            "bus",
            changed_anchor,
            first.run().clone(),
        )
        .expect("changed receipt should reconstruct");

        assert_ne!(first.commitment(), changed.commitment());
    }

    #[test]
    fn persisted_receipt_can_be_reconstructed_without_executable_authority() {
        let receipt = ProcessExecutionReceipt::from_persisted_parts(
            "exec-persisted",
            "regolith_electrolysis",
            "feed-persisted",
            "waste",
            10,
            20,
            "bus",
            ExecutionStateAnchor::in_memory(),
            ProcessRun::new(
                "regolith_electrolysis",
                "regolith",
                "feed-persisted",
                1_000,
                BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
                100,
                4_000,
            ),
        )
        .expect("persisted receipt should reconstruct");

        assert_eq!(receipt.execution_id(), "exec-persisted");
        assert_eq!(receipt.process_id(), "regolith_electrolysis");
        assert_eq!(receipt.input_batch_id(), "feed-persisted");
        assert_eq!(receipt.waste_stream(), "waste");
        assert_eq!(receipt.first_inventory_sequence(), 10);
        assert_eq!(receipt.energy_sequence(), 20);
        assert_eq!(receipt.energy_node_id(), "bus");
        assert_eq!(receipt.state_anchor().frontier(), "UNANCHORED");
    }

    #[test]
    fn persisted_pending_receipt_can_restore_pre_crash_reservation() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-restore",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut original_budget = ExecutionBudget::new(1_000, 4_000);
        let mut original_inventory =
            InventoryLedger::new(BTreeMap::from([("feed-restore".to_string(), 1_000)]));
        let receipt = process
            .authorize_pending_execution_with_inventory(
                "exec-restore",
                10,
                20,
                "bus",
                run,
                &mut original_budget,
                &mut original_inventory,
            )
            .expect("pending execution should authorize");

        let persisted = ProcessExecutionReceipt::from_persisted_parts(
            receipt.execution_id(),
            receipt.process_id(),
            receipt.input_batch_id(),
            receipt.waste_stream(),
            receipt.first_inventory_sequence(),
            receipt.energy_sequence(),
            receipt.energy_node_id(),
            ExecutionStateAnchor::in_memory(),
            receipt.run().clone(),
        )
        .expect("persisted receipt should reconstruct exactly");

        let mut recovered_budget = ExecutionBudget::new(1_000, 4_000);
        let mut recovered_inventory =
            InventoryLedger::new(BTreeMap::from([("feed-restore".to_string(), 1_000)]));
        process
            .restore_pending_execution_with_inventory(
                &persisted,
                &mut recovered_budget,
                &mut recovered_inventory,
            )
            .expect("persisted pending receipt should restore its reservation");

        assert_eq!(
            recovered_budget
                .execution_record("exec-restore")
                .expect("restored pending record should exist")
                .receipt(),
            &receipt
        );
        assert_eq!(recovered_budget.available_feed_mass_g(), 0);
        assert_eq!(recovered_budget.available_energy_units(), 0);

        let recovered =
            resume_pending_execution("exec-restore", &recovered_budget, &recovered_inventory)
                .expect("restored pending execution should activate");
        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5_000)]));
        commit_process_execution(
            &recovered,
            &mut recovered_budget,
            &mut recovered_inventory,
            &mut energy,
        )
        .expect("restored execution should commit");
        assert_eq!(
            recovered_budget.execution_state("exec-restore"),
            Some(ExecutionState::Committed)
        );
    }

    #[test]
    fn pending_activation_requires_explicit_pending_receipt() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-activate",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-activate".to_string(), 1_000)]));
        let pending = process
            .authorize_pending_execution_with_inventory(
                "exec-activate",
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("pending execution should authorize");

        assert_eq!(
            budget
                .execution_record("exec-activate")
                .expect("pending record should exist")
                .receipt(),
            &pending
        );

        let activated = resume_pending_execution("exec-activate", &budget, &inventory)
            .expect("explicit activation should rehydrate executable proof");
        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5_000)]));
        commit_process_execution(&activated, &mut budget, &mut inventory, &mut energy)
            .expect("activated execution should commit");
        assert_eq!(
            budget.execution_state("exec-activate"),
            Some(ExecutionState::Committed)
        );
    }

    #[test]
    fn execution_record_exposes_complete_receipt_identity() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-record",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-record".to_string(), 1_000)]));
        process
            .authorize_execution_with_inventory(
                "exec-record",
                10,
                20,
                "bus",
                run.clone(),
                &mut budget,
                &mut inventory,
            )
            .expect("execution should authorize");

        let record = budget
            .execution_record("exec-record")
            .expect("pending record should be available");
        let receipt = record.receipt();

        assert_eq!(receipt.execution_id(), "exec-record");
        assert_eq!(receipt.process_id(), "regolith_electrolysis");
        assert_eq!(receipt.input_batch_id(), "feed-record");
        assert_eq!(receipt.waste_stream(), "waste");
        assert_eq!(receipt.first_inventory_sequence(), 10);
        assert_eq!(receipt.energy_sequence(), 20);
        assert_eq!(receipt.energy_node_id(), "bus");
        assert_eq!(receipt.run(), &run);
    }

    #[test]
    fn pending_execution_can_rehydrate_executable_proof() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-rehydrate",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-rehydrate".to_string(), 1_000)]));
        let receipt = process
            .authorize_execution_with_inventory(
                "exec-rehydrate",
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("execution should authorize");
        let execution_id = receipt.execution_id().to_string();
        drop(receipt);

        let recovered = resume_pending_execution(&execution_id, &budget, &inventory)
            .expect("pending execution should rehydrate");

        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5_000)]));
        commit_process_execution(&recovered, &mut budget, &mut inventory, &mut energy)
            .expect("rehydrated execution should commit");
        assert_eq!(inventory.source_reservations.len(), 0);
        assert_eq!(inventory.events().len(), 4);
        assert_eq!(energy.events().len(), 1);
    }

    #[test]
    fn aborted_execution_restores_budget_without_reusing_identity() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-abort",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-abort".to_string(), 1_000)]));
        let receipt = process
            .authorize_execution_with_inventory(
                "exec-abort",
                10,
                20,
                "bus",
                run.clone(),
                &mut budget,
                &mut inventory,
            )
            .expect("execution should authorize");

        assert_eq!(budget.available_feed_mass_g(), 0);
        assert_eq!(budget.available_energy_units(), 0);

        abort_process_execution(&receipt, &mut budget, &mut inventory)
            .expect("abort should restore reservation");

        assert_eq!(budget.available_feed_mass_g(), 1_000);
        assert_eq!(budget.available_energy_units(), 4_000);
        assert!(abort_process_execution(&receipt, &mut budget, &mut inventory).is_err());

        assert!(
            process
                .authorize_execution("exec-abort", 30, 40, "bus", run.clone(), &mut budget)
                .is_err()
        );
        assert!(
            process
                .authorize_execution("exec-reuse-different-id", 30, 40, "bus", run, &mut budget)
                .is_ok()
        );
    }

    #[test]
    fn failed_cross_ledger_commit_does_not_partially_consume_state() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-transaction-fail",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory = InventoryLedger::new(BTreeMap::from([(
            "feed-transaction-fail".to_string(),
            1_000,
        )]));
        let receipt = process
            .authorize_execution_with_inventory(
                "exec-transaction-fail",
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("execution should authorize");
        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 3_000)]));

        assert!(
            commit_process_execution(&receipt, &mut budget, &mut inventory, &mut energy).is_err()
        );

        assert_eq!(inventory.state().get("feed-transaction-fail"), Some(&1_000));
        assert!(inventory.events().is_empty());
        assert_eq!(inventory.source_reservations.len(), 1);
        assert_eq!(energy.state().get("bus"), Some(&3_000));
        assert!(energy.events().is_empty());
        assert_eq!(budget.available_feed_mass_g(), 0);
        assert_eq!(budget.available_energy_units(), 0);

        energy
            .append(
                EnergyEvent::new(21, "bus", 4_000, EnergyEventKind::Generated)
                    .with_event_id("recovery-generation")
                    .with_provenance("recovery"),
            )
            .expect("restoring energy should be explicit");

        commit_process_execution(&receipt, &mut budget, &mut inventory, &mut energy)
            .expect("pending receipt should remain retryable");

        assert_eq!(inventory.state().get("feed-transaction-fail"), Some(&0));
        assert_eq!(inventory.source_reservations.len(), 0);
        assert_eq!(energy.state().get("bus"), Some(&3_000));
        assert_eq!(inventory.events().len(), 4);
        assert_eq!(energy.events().len(), 2);
    }

    #[test]
    fn estimated_resource_cannot_enter_inventory_without_certification() {
        let claim = ResourceClaim::new(
            "mercury-polar-ice",
            5_000,
            EvidenceGrade::RemoteObserved,
            950_000,
        )        .expect("valid resource claim");

        assert!(
            claim
                .certify_for_inventory(EvidenceGrade::InSituMeasured, 900_000)
                .is_err()
        );

        let measured = ResourceClaim::new(
            "mercury-polar-ice",
            5_000,
            EvidenceGrade::InSituMeasured,
            950_000,
        )        .expect("valid resource claim");

        let certified = measured
            .certify_for_inventory(EvidenceGrade::InSituMeasured, 900_000)
            .expect("measured claim with sufficient confidence should certify");
        assert_eq!(
            certified.certificate_id(),
            "resource-certificate:mercury-polar-ice"
        );
        assert_eq!(certified.claim_id(), "mercury-polar-ice");
        assert_eq!(certified.mass_g(), 5_000);

        let event = InventoryEvent::from_certified_resource(20, "ice-batch-001", &certified);
        let replayed = InventoryEvent::from_certified_resource(21, "ice-batch-002", &certified);

        assert_eq!(event.kind, InventoryEventKind::Produced);
        assert_eq!(event.mass_g, 5_000);
        assert_eq!(
            event.event_id.as_deref(),
            Some("resource-certificate:mercury-polar-ice")
        );
        assert_eq!(event.provenance_id.as_deref(), Some("mercury-polar-ice"));
        assert_eq!(replayed.event_id, event.event_id);
        assert!(replay_inventory(&BTreeMap::new(), &[event, replayed]).is_err());
    }

    #[test]
    fn resource_claim_requires_identity_before_certification() {
        assert!(
            ResourceClaim::new("", 5_000, EvidenceGrade::InSituMeasured, 950_000).is_err()
        );
    }

    #[test]
    fn resource_claim_rejects_out_of_range_confidence() {
        assert!(
            ResourceClaim::new(
                "mercury-polar-ice",
                5_000,
                EvidenceGrade::InSituMeasured,
                1_000_001,
            )
            .is_err()
        );
        assert!(
            ResourceClaim::new(
                "mercury-polar-ice",
                5_000,
                EvidenceGrade::InSituMeasured,
                1_000_000,
            )
            .is_ok()
        );
    }

    #[test]
    fn process_imbalance_is_rejected() {
        let run = ProcessRun::new(
            "broken_refinery",
            "ore",
            "feed-002",
            1_000,
            BTreeMap::from([("metal".to_string(), 800)]),
            100,
            2_000,
        );

        assert!(run.validate_mass_balance().is_err());
    }

    #[test]
    fn process_efficiency_remains_exact_at_u64_bounds() {
        let lower = ProcessEfficiency::new(u64::MAX, u64::MAX, u64::MAX);
        let higher = ProcessEfficiency::new(u64::MAX, u64::MAX - 1, u64::MAX);

        assert!(higher.better_than(lower));
        assert!(!lower.better_than(higher));
    }

    #[test]
    fn process_efficiency_ties_remain_ties() {
        let left = ProcessEfficiency::new(u64::MAX, u64::MAX, 0);
        let right = ProcessEfficiency::new(u64::MAX, u64::MAX - 1, 1);

        assert!(!left.better_than(right));
        assert!(!right.better_than(left));
    }

    #[test]
    fn process_efficiency_is_deterministic() {
        let focused = ProcessEfficiency::new(10, 100, 100);
        let diffuse = ProcessEfficiency::new(12, 200, 200);

        assert!(focused.better_than(diffuse));
        assert!(!diffuse.better_than(focused));
    }

    #[test]
    fn bootstrap_candidate_rejects_invalid_identity_and_risk() {
        assert!(
            BootstrapCandidate::new("", 1, 1, 1, 1, 1, 0).is_err()
        );
        assert!(
            BootstrapCandidate::new("candidate", 1, 1, 1, 1, 1, 1_000_001).is_err()
        );
        assert!(
            BootstrapCandidate::new("candidate", 1, 1, 1, 1, 1, 1_000_000).is_ok()
        );
    }

    #[test]
    fn pareto_frontier_prefers_dependency_closure_without_single_score() {
        let dependency_remover = BootstrapCandidate::new(
            "close_electronics",
            30,
            30,
            100,
            500,
            20,
            50_000,
        )
        .expect("valid bootstrap candidate");
        let throughput = BootstrapCandidate::new(
            "increase_bulk_output",
            0,
            0,
            100,
            500,
            20,
            50_000,
        )
        .expect("valid bootstrap candidate");
        let frontier = pareto_frontier(&[throughput, dependency_remover]);

        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].id, "close_electronics");
    }

    #[test]
    fn pareto_frontier_preserves_real_tradeoffs() {
        let low_energy =
            BootstrapCandidate::new("low_energy", 10, 5, 100, 100, 30, 100_000)
                .expect("valid bootstrap candidate");
        let low_mass =
            BootstrapCandidate::new("low_mass", 10, 5, 50, 200, 30, 100_000)
                .expect("valid bootstrap candidate");
        let frontier = pareto_frontier(&[low_energy, low_mass]);

        assert_eq!(frontier.len(), 2);
        assert_eq!(
            frontier
                .iter()
                .map(|candidate| candidate.id.as_str())
                .collect::<Vec<_>>(),
            vec!["low_energy", "low_mass"]
        );
    }

    #[test]
    fn pareto_frontier_compares_same_id_observations() {
        let dominated =
            BootstrapCandidate::new("same_id", 1, 1, 100, 100, 30, 100_000)
                .expect("valid bootstrap candidate");
        let stronger =
            BootstrapCandidate::new("same_id", 2, 2, 90, 90, 20, 90_000)
                .expect("valid bootstrap candidate");
        let frontier = pareto_frontier(&[dominated, stronger]);

        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].critical_weight_closed_gain, 2);
    }

    #[test]
    fn pareto_frontier_is_input_order_independent() {
        let a =
            BootstrapCandidate::new("a", 10, 5, 100, 100, 30, 100_000).expect("valid bootstrap candidate");
        let b =
            BootstrapCandidate::new("b", 20, 5, 100, 100, 30, 100_000).expect("valid bootstrap candidate");
        let c =
            BootstrapCandidate::new("c", 5, 10, 90, 110, 20, 80_000).expect("valid bootstrap candidate");

        let first = pareto_frontier(&[a.clone(), b.clone(), c.clone()]);
        let second = pareto_frontier(&[c, b, a]);

        assert_eq!(first, second);
        assert!(first.iter().any(|candidate| candidate.id == "b"));
        assert!(first.iter().any(|candidate| candidate.id == "c"));
        assert!(!first.iter().any(|candidate| candidate.id == "a"));
    }

    #[test]
    fn bootstrap_efficiency_compares_exactly() {
        let better = BootstrapEfficiency::new(3, 2);
        let worse = BootstrapEfficiency::new(4, 3);

        assert!(better.better_than(worse));
        assert!(!worse.better_than(better));
    }

    #[test]
    fn recovery_horizon_is_explicit() {
        assert_eq!(recovery_horizon(&RecoveryOutcome::Immediate), Some(0));
        assert_eq!(
            recovery_horizon(&RecoveryOutcome::RecoveredAfter { ticks: 17 }),
            Some(17)
        );
        assert_eq!(recovery_horizon(&RecoveryOutcome::Unrecoverable), None);
    }

    #[test]
    fn inventory_replay_is_causal_and_order_independent() {
        let initial = BTreeMap::from([("scrap".to_string(), 10), ("steel".to_string(), 100)]);
        let events = [
            InventoryEvent::new(1, "steel", 25, InventoryEventKind::Produced)
                .with_event_id("forge-run-1:1")
                .with_provenance("forge-run-1"),
            InventoryEvent::new(2, "steel", 60, InventoryEventKind::Consumed)
                .with_event_id("forge-run-2:2")
                .with_provenance("forge-run-2"),
            InventoryEvent::new(3, "scrap", 10, InventoryEventKind::Consumed)
                .with_event_id("recycle-run-1:consume")
                .with_provenance("recycle-run-1"),
            InventoryEvent::new(4, "steel", 10, InventoryEventKind::Produced)
                .with_event_id("recycle-run-1:produce")
                .with_provenance("recycle-run-1"),
        ];
        let expected = BTreeMap::from([("scrap".to_string(), 0), ("steel".to_string(), 75)]);

        verify_inventory_conservation(&initial, &events, &expected)
            .expect("causal inventory replay must match");

        let reordered = [
            events[3].clone(),
            events[2].clone(),
            events[0].clone(),
            events[1].clone(),
        ];
        assert_eq!(
            replay_inventory(&initial, &reordered).expect("replay must succeed"),
            expected
        );
    }

    #[test]
    fn recovery_chain_requires_an_explicit_source_consumption() {
        let unfunded = BTreeMap::from([("scrap".to_string(), 9)]);
        let recovery = [
            InventoryEvent::new(1, "scrap", 10, InventoryEventKind::Consumed)
                .with_event_id("recycle-valid:consume")
                .with_provenance("recycle-valid"),
            InventoryEvent::new(2, "steel", 10, InventoryEventKind::Produced)
                .with_event_id("recycle-valid:produce")
                .with_provenance("recycle-valid"),
        ];
        assert!(replay_inventory(&unfunded, &recovery).is_err());

        let funded = BTreeMap::from([("scrap".to_string(), 10)]);
        let recovered = replay_inventory(&funded, &recovery)
            .expect("recycling must consume a real source batch");
        assert_eq!(recovered.get("scrap"), Some(&0));
        assert_eq!(recovered.get("steel"), Some(&10));
    }

    #[test]
    fn stateful_inventory_ledger_rejects_partial_batch_without_mutation() {
        let mut ledger = InventoryLedger::new(BTreeMap::from([("steel".to_string(), 10)]));
        let batch = [
            InventoryEvent::new(1, "steel", 4, InventoryEventKind::Consumed)
                .with_event_id("inventory-1")
                .with_provenance("run"),
            InventoryEvent::new(2, "steel", 10, InventoryEventKind::Consumed)
                .with_event_id("inventory-2")
                .with_provenance("run"),
        ];

        assert!(ledger.append_batch(&batch).is_err());
        assert_eq!(ledger.state().get("steel"), Some(&10));
        assert!(ledger.events().is_empty());
    }

    #[test]
    fn stateful_energy_ledger_rejects_partial_batch_without_mutation() {
        let mut ledger = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5)]));
        let batch = [
            EnergyEvent::new(1, "bus", 2, EnergyEventKind::Consumed)
                .with_event_id("energy-1")
                .with_provenance("run"),
            EnergyEvent::new(2, "bus", 10, EnergyEventKind::Consumed)
                .with_event_id("energy-2")
                .with_provenance("run"),
        ];

        assert!(ledger.append_batch(&batch).is_err());
        assert_eq!(ledger.state().get("bus"), Some(&5));
        assert!(ledger.events().is_empty());
    }

    #[test]
    fn stateful_inventory_ledger_rejects_duplicate_ids_across_appends() {
        let mut ledger = InventoryLedger::new(BTreeMap::from([("steel".to_string(), 10)]));
        let first = InventoryEvent::new(1, "steel", 5, InventoryEventKind::Consumed)
            .with_event_id("inventory-1")
            .with_provenance("run-1");
        ledger
            .append(first.clone())
            .expect("first append should succeed");

        let duplicate = InventoryEvent::new(2, "steel", 5, InventoryEventKind::Consumed)
            .with_event_id("inventory-1")
            .with_provenance("run-1");
        assert!(ledger.append(duplicate).is_err());
        assert_eq!(ledger.state().get("steel"), Some(&5));
        assert_eq!(ledger.events(), &[first]);
    }

    #[test]
    fn stateful_inventory_ledger_rejects_non_monotonic_sequence_without_mutation() {
        let mut ledger = InventoryLedger::new(BTreeMap::from([("steel".to_string(), 10)]));
        ledger
            .append(
                InventoryEvent::new(5, "steel", 2, InventoryEventKind::Consumed)
                    .with_event_id("inventory-5")
                    .with_provenance("run"),
            )
            .expect("first append should succeed");

        assert!(
            ledger
                .append(
                    InventoryEvent::new(4, "steel", 3, InventoryEventKind::Consumed)
                        .with_event_id("inventory-4")
                        .with_provenance("run"),
                )
                .is_err()
        );

        assert_eq!(ledger.state().get("steel"), Some(&8));
        assert_eq!(ledger.events().len(), 1);
    }

    #[test]
    fn stateful_energy_ledger_rejects_duplicate_ids_across_appends() {
        let mut ledger = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 10)]));
        ledger
            .append(
                EnergyEvent::new(1, "bus", 4, EnergyEventKind::Consumed)
                    .with_event_id("energy-1")
                    .with_provenance("run-1"),
            )
            .expect("first append should succeed");

        assert!(
            ledger
                .append(
                    EnergyEvent::new(2, "bus", 4, EnergyEventKind::Consumed)
                        .with_event_id("energy-1")
                        .with_provenance("run-1"),
                )
                .is_err()
        );

        assert_eq!(ledger.state().get("bus"), Some(&6));
        assert_eq!(ledger.events().len(), 1);
    }

    #[test]
    fn stateful_energy_ledger_rejects_failed_append_without_mutation() {
        let mut ledger = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5)]));
        let accepted = EnergyEvent::new(1, "bus", 2, EnergyEventKind::Consumed)
            .with_event_id("energy-accepted")
            .with_provenance("run");
        ledger
            .append(accepted)
            .expect("first append should succeed");

        let rejected = EnergyEvent::new(2, "bus", 10, EnergyEventKind::Consumed)
            .with_event_id("energy-rejected")
            .with_provenance("run");
        assert!(ledger.append(rejected).is_err());
        assert_eq!(ledger.state().get("bus"), Some(&3));
        assert_eq!(ledger.events().len(), 1);
    }

    #[test]
    fn inventory_replay_rejects_bad_history() {
        let initial = BTreeMap::from([("steel".to_string(), 1)]);

        let overdraw = [
            InventoryEvent::new(1, "steel", 2, InventoryEventKind::Consumed)
                .with_event_id("overdraw:1")
                .with_provenance("test"),
        ];
        assert!(replay_inventory(&initial, &overdraw).is_err());

        let duplicate = [
            InventoryEvent::new(1, "a", 1, InventoryEventKind::Produced)
                .with_event_id("a:1")
                .with_provenance("a"),
            InventoryEvent::new(1, "b", 1, InventoryEventKind::Produced)
                .with_event_id("b:1")
                .with_provenance("b"),
        ];
        assert!(replay_inventory(&BTreeMap::new(), &duplicate).is_err());
    }

    #[test]
    fn ledgers_reject_duplicate_event_ids_even_at_distinct_sequences() {
        let inventory = [
            InventoryEvent::new(1, "steel", 1, InventoryEventKind::Produced)
                .with_event_id("same-event")
                .with_provenance("run"),
            InventoryEvent::new(2, "steel", 1, InventoryEventKind::Produced)
                .with_event_id("same-event")
                .with_provenance("run"),
        ];
        assert!(replay_inventory(&BTreeMap::new(), &inventory).is_err());

        let energy = [
            EnergyEvent::new(1, "bus", 1, EnergyEventKind::Generated)
                .with_event_id("same-event")
                .with_provenance("run"),
            EnergyEvent::new(2, "bus", 1, EnergyEventKind::Generated)
                .with_event_id("same-event")
                .with_provenance("run"),
        ];
        assert!(replay_energy(&BTreeMap::new(), &energy).is_err());
    }

    #[test]
    fn receipt_reemission_is_detectable_by_event_identity() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-replay",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );
        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed-replay".to_string(), 1_000)]));
        let receipt = process
            .authorize_execution_with_inventory(
                "exec-replay",
                70,
                71,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("execution should authorize");

        let first = receipt
            .inventory_events()
            .expect("first event materialization");
        let second = receipt
            .inventory_events()
            .expect("second event materialization");
        let mut combined = first;
        combined.extend(second);
        assert!(
            replay_inventory(
                &BTreeMap::from([("feed-replay".to_string(), 1_000)]),
                &combined
            )
            .is_err()
        );

        let energy = receipt.energy_event();
        assert!(
            replay_energy(
                &BTreeMap::from([("bus".to_string(), 8_000)]),
                &[energy.clone(), energy],
            )
            .is_err()
        );
    }

    #[test]
    fn ledgers_reject_unprovenanced_positive_events() {
        let inventory = [InventoryEvent::new(
            1,
            "steel",
            1,
            InventoryEventKind::Produced,
        )];
        assert!(replay_inventory(&BTreeMap::new(), &inventory).is_err());

        let energy = [EnergyEvent::new(1, "bus", 1, EnergyEventKind::Generated)];
        assert!(replay_energy(&BTreeMap::new(), &energy).is_err());
    }

    #[test]
    fn energy_commitment_binds_history_not_only_balance() {
        let mut left = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 10)]));
        let mut right = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 10)]));

        left.append(
            EnergyEvent::new(1, "bus", 3, EnergyEventKind::Consumed)
                .with_event_id("energy-a")
                .with_provenance("run-a"),
        ).expect("left event");
        right.append(
            EnergyEvent::new(1, "bus", 3, EnergyEventKind::Consumed)
                .with_event_id("energy-b")
                .with_provenance("run-b"),
        ).expect("right event");

        assert_eq!(left.state(), right.state());
        assert_ne!(left.events(), right.events());
        assert_ne!(left.state_commitment(), right.state_commitment());
    }

    #[test]
    fn receipt_causal_event_ids_are_deterministic() {
        let process =
            ProductionProcess::new("electrolysis", "regolith", ["oxygen", "metal"], "slag");
        let run = ProcessRun::new(
            "electrolysis",
            "regolith",
            "feed",
            1_000,
            BTreeMap::from([("metal".to_string(), 720), ("oxygen".to_string(), 180)]),
            100,
            4_000,
        );
        let mut budget = ExecutionBudget::new(1_000, 4_000);
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("feed".to_string(), 1_000)]));
        let receipt = process
            .authorize_pending_execution_with_inventory(
                "exec-ids",
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
            )
            .expect("pending authorization");

        assert_eq!(
            receipt.inventory_event_ids(),
            vec![
                "exec-ids:inventory:consume",
                "exec-ids:inventory:produce:metal",
                "exec-ids:inventory:produce:oxygen",
                "exec-ids:inventory:waste:slag",
            ]
        );
        assert_eq!(receipt.energy_event_id(), "exec-ids:energy:consume");
    }

    #[test]
    fn ratio_ppm_is_bounded_and_deterministic() {
        assert_eq!(ratio_ppm(1, 0), 0);
        assert_eq!(ratio_ppm(1, 2), 500_000);
        assert_eq!(ratio_ppm(2, 1), 1_000_000);
    }
}
