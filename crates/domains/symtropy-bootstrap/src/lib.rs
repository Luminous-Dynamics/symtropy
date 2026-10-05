// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic kernel for space-bootstrap and industrial-closure reasoning.
//!
//! This crate intentionally has no Bevy, Holochain, Symthaea, or external
//! simulation dependencies. It turns industrial closure into explicit,
//! replayable state and graph calculations that higher layers can consume.

use std::collections::{BTreeMap, BTreeSet};

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

        let weighted_critical_total =
            self.capabilities
                .values()
                .try_fold(0_u64, |sum, capability| {
                    sum.checked_add(capability.critical_weight)
                });

        if weighted_critical_total.is_none() {
            report_definition_errors.push("critical weight sum overflow".to_string());
        }

        let weighted_critical_closed =
            assessments
                .iter()
                .filter(|assessment| assessment.closed)
                .filter_map(|assessment| self.capabilities.get(&assessment.id))
                .try_fold(0_u64, |sum, capability| {
                    sum.checked_add(capability.critical_weight)
                });

        if weighted_critical_closed.is_none() {
            report_definition_errors.push("closed critical weight sum overflow".to_string());
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
                ratio_ppm(local_mass_g, local_mass_g.saturating_add(imported_mass_g))
            } else {
                0
            },
            valid,
            definition_errors: report_definition_errors,
        }
    }

    /// Rank unresolved dependencies by the criticality weight they block.
    #[must_use]
    pub fn rank_blockers(&self, report: &ClosureReport) -> Vec<DependencyBlocker> {
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
                entry.0 = entry.0.saturating_add(capability.critical_weight);
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

        ranked
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

/// Consumable authorization budget for one deterministic execution scope.
///
/// Feedstock and energy are reserved exactly once when an execution receipt is
/// minted. The reservation state is private so callers cannot restore capacity
/// without creating a new budget scope.
#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionBudget {
    available_feed_mass_g: u64,
    available_energy_units: u64,
    reserved_executions: BTreeMap<String, ProcessExecutionReceipt>,
    settled_execution_ids: BTreeSet<String>,
}

impl ExecutionBudget {
    #[must_use]
    pub const fn new(available_feed_mass_g: u64, available_energy_units: u64) -> Self {
        Self {
            available_feed_mass_g,
            available_energy_units,
            reserved_executions: BTreeMap::new(),
            settled_execution_ids: BTreeSet::new(),
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

    /// Reserve capacity for the complete receipt, not just its quantities.
    ///
    /// Binding the reservation to the immutable receipt prevents a future
    /// caller or internal refactor from swapping process, batch, sequence, node,
    /// or run parameters while retaining the same funded execution identity.
    fn reserve(&mut self, receipt: &ProcessExecutionReceipt) -> Result<(), String> {
        let execution_id = receipt.execution_id();

        if self.reserved_executions.contains_key(execution_id)
            || self.settled_execution_ids.contains(execution_id)
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

    fn settle(&mut self, execution_id: &str) -> Result<(), String> {
        if self.reserved_executions.remove(execution_id).is_none() {
            return Err(format!("execution is not pending: {execution_id}"));
        }

        self.settled_execution_ids.insert(execution_id.to_string());
        Ok(())
    }

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

        self.settled_execution_ids.insert(execution_id.to_string());
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

    /// Authorize an executable process against aggregate budgets and its concrete source batch.
    ///
    /// The physical source reservation is acquired before budget reservation. A
    /// failed budget reservation releases the source claim so the operation is
    /// atomic across both admission boundaries.
    pub fn authorize_execution_with_inventory(
        &self,
        execution_id: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
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

    /// Atomically authorize one process execution against an aggregate consumable budget.
    ///
    /// This budget-only path does not reserve physical source inventory; receipts created
    /// here are suitable for budgeting/provenance inspection but are not executable through
    /// `commit_process_execution`. Use `authorize_execution_with_inventory` for execution.
    ///
    /// Successful authorization mints an immutable receipt. Material and energy
    /// events must be derived from that receipt rather than from a raw budget
    /// snapshot, preventing the same provisioned capacity from being authorized
    /// twice within the budget scope.
    pub fn authorize_execution(
        &self,
        execution_id: impl Into<String>,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
    ) -> Result<ProcessExecutionReceipt, String> {
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
            run,
        };

        budget.reserve(&receipt)?;
        Ok(receipt)
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

    #[must_use]
    pub fn total_output_mass_g(&self) -> u64 {
        self.output_mass_g.values().copied().sum()
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
    run: ProcessRun,
}

impl ProcessExecutionReceipt {
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
    /// Waste is emitted as an explicit produced batch rather than disappearing
    /// from the material ledger. Any later disposal or recovery must consume
    /// that named waste batch explicitly.
    pub fn inventory_events(&self) -> Result<Vec<InventoryEvent>, String> {
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
    #[must_use]
    pub fn energy_event(&self) -> EnergyEvent {
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

/// Commit one authorized process execution across budget, inventory, and energy.
///
/// Both ledgers are staged and fully validated before either live ledger is
/// replaced. The budget reservation is settled only after both staged ledgers
/// succeed, so a failed commit remains retryable without partial state.
pub fn commit_process_execution(
    receipt: &ProcessExecutionReceipt,
    budget: &mut ExecutionBudget,
    inventory: &mut InventoryLedger,
    energy: &mut EnergyLedger,
) -> Result<(), String> {
    let reservation = budget
        .reservation(receipt.execution_id())
        .ok_or_else(|| format!("execution is not pending: {}", receipt.execution_id()))?;

    if reservation != receipt {
        return Err("execution reservation does not match receipt".to_string());
    }

    let inventory_events = receipt.inventory_events()?;
    let energy_event = receipt.energy_event();

    let mut staged_inventory = inventory.clone();
    staged_inventory.append_reserved_process_batch(
        receipt.execution_id(),
        receipt.input_batch_id(),
        receipt.feed_mass_g(),
        &inventory_events,
    )?;

    let mut staged_energy = energy.clone();
    staged_energy.append_batch(std::slice::from_ref(&energy_event))?;

    *inventory = staged_inventory;
    *energy = staged_energy;
    budget.settle(receipt.execution_id())?;

    Ok(())
}

/// Abort one authorized process execution without mutating either ledger.
///
/// The reserved feedstock and energy are returned to the budget, while the
/// execution identity becomes terminal so the same authorization cannot be
/// silently reused. This is the explicit rollback path for a prepared execution
/// that will not be committed.
pub fn abort_process_execution(
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
    budget.abort(receipt.execution_id())?;
    *inventory = staged_inventory;

    Ok(())
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

    /// Compare capability gained per combined mass-energy burden.
    ///
    /// This is intentionally a structural comparator rather than a physical
    /// economic claim; higher layers decide the actual weighting of resources.
    #[must_use]
    pub fn better_than(self, other: Self) -> bool {
        let lhs = (self.capability_gain as u128)
            .saturating_mul(other.feed_mass_g.saturating_add(other.energy_units) as u128);
        let rhs = (other.capability_gain as u128)
            .saturating_mul(self.feed_mass_g.saturating_add(self.energy_units) as u128);
        lhs > rhs
    }
}

/// A candidate industrial transition represented by independently auditable dimensions.
///
/// The frontier deliberately avoids collapsing capability gain, imported mass,
/// energy, time, and failure risk into one scalar. Higher-is-better dimensions
/// are maximized; burden dimensions are minimized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapCandidate {
    pub id: String,
    pub critical_weight_closed_gain: u64,
    pub dependency_weight_removed: u64,
    pub imported_mass_g: u64,
    pub energy_units: u64,
    pub time_ticks: u64,
    pub failure_risk_ppm: u64,
}

impl BootstrapCandidate {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        critical_weight_closed_gain: u64,
        dependency_weight_removed: u64,
        imported_mass_g: u64,
        energy_units: u64,
        time_ticks: u64,
        failure_risk_ppm: u64,
    ) -> Self {
        Self {
            id: id.into(),
            critical_weight_closed_gain,
            dependency_weight_removed,
            imported_mass_g,
            energy_units,
            time_ticks,
            failure_risk_ppm: failure_risk_ppm.min(1_000_000),
        }
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
            source_reservations: BTreeMap::new(),
            seen_event_ids: BTreeSet::new(),
            last_sequence: None,
        }
    }

    /// Reserve a concrete source batch for one pending execution.
    ///
    /// Reservations are physical-stock claims separate from aggregate budget
    /// authorization. A reserved batch cannot be consumed by another append
    /// until the owning execution commits or releases it.
    pub fn reserve_source_batch(
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
            || matching_consumes[0].provenance_id.as_deref()
                != Some(expected_provenance.as_str())
        {
            return Err("reserved source batch is not consumed exactly by its execution".to_string());
        }

        let mut staged = self.clone();
        staged.release_source_batch(execution_id)?;
        staged.append_batch(events)?;
        *self = staged;

        Ok(())
    }

    /// Append one event atomically.
    pub fn append(&mut self, event: EnergyEvent) -> Result<(), String> {
        self.append_batch(std::slice::from_ref(&event))
    }

    /// Validate and commit a complete event batch atomically.
    pub fn append_batch(&mut self, events: &[EnergyEvent]) -> Result<(), String> {
        let mut staged_balances = BTreeMap::new();
        let mut staged_event_ids = BTreeSet::new();
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

            if self.seen_event_ids.contains(event_id) || !staged_event_ids.insert(event_id) {
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
    pub id: String,
    pub mass_g: u64,
    pub evidence: EvidenceGrade,
    pub confidence_ppm: u64,
}

impl ResourceClaim {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        mass_g: u64,
        evidence: EvidenceGrade,
        confidence_ppm: u64,
    ) -> Self {
        Self {
            id: id.into(),
            mass_g,
            evidence,
            confidence_ppm: confidence_ppm.min(1_000_000),
        }
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

        if self.evidence < minimum_evidence {
            return Err(format!(
                "insufficient evidence grade: required={minimum_evidence:?}, observed={:?}",
                self.evidence
            ));
        }

        let minimum_confidence_ppm = minimum_confidence_ppm.min(1_000_000);
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
        let mut staged_event_ids = BTreeSet::new();
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

            if self.seen_event_ids.contains(event_id) || !staged_event_ids.insert(event_id) {
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
        ((((numerator as u128) * 1_000_000u128) / (denominator as u128)).min(1_000_000)) as u64
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
    fn unknown_dependency_fails_closed() {
        let graph = DependencyGraph::new([Capability::new("rover", 100, ["mystery_part"])], []);
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
    fn critical_weight_sum_overflow_fails_closed() {
        let graph = DependencyGraph::new(
            [
                Capability::new("a", u64::MAX, []),
                Capability::new("b", 1, []),
            ],
            [],
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
            [Capability::new("seed", 100, [])],
            [],
        );
        let report = graph.evaluate(100, 0);

        let requirements = [StageRequirement::new(ClosureStage::Seed, [])];
        assert_eq!(highest_closed_stage(&report, &requirements), None);
    }

    #[test]
    fn duplicate_stage_requirements_fail_closed() {
        let graph = DependencyGraph::new(
            [Capability::new("seed", 100, [])],
            [],
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
            [],
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
            [],
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
            [Capability::new("steel", 100, [])],
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
            [Capability::new("", 10, [])],
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
        let blockers = graph.rank_blockers(&report);

        assert_eq!(blockers[0].id, "electronics");
        assert_eq!(blockers[0].weight, 30);
        assert_eq!(blockers[0].affected_capabilities, vec!["controller"]);
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
        assert_eq!(run.total_output_mass_g(), 900);
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

        let collision = ProductionProcess::new(
            "waste-collision",
            "regolith",
            ["oxygen", "waste"],
            "waste",
        );
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
    fn failed_budget_authorization_releases_source_reservation() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let too_expensive = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "release-feed",
            500,
            BTreeMap::from([("oxygen".to_string(), 90), ("metal".to_string(), 360)]),
            50,
            1_000,
        );
        let affordable = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "release-feed",
            500,
            BTreeMap::from([("oxygen".to_string(), 90), ("metal".to_string(), 360)]),
            50,
            500,
        );

        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("release-feed".to_string(), 500)]));
        let mut budget = ExecutionBudget::new(500, 500);

        assert!(
            process
                .authorize_execution_with_inventory(
                    "exec-release",
                    1,
                    2,
                    "bus",
                    too_expensive,
                    &mut budget,
                    &mut inventory,
                )
                .is_err()
        );

        assert_eq!(budget.available_feed_mass_g(), 500);
        assert_eq!(budget.available_energy_units(), 500);

        process
            .authorize_execution_with_inventory(
                "exec-release",
                1,
                2,
                "bus",
                affordable,
                &mut budget,
                &mut inventory,
            )
            .expect("released source reservation should allow retry");
    }

    #[test]
    fn reserved_source_blocks_unrelated_inventory_consumption() {
        let mut inventory =
            InventoryLedger::new(BTreeMap::from([("ore-batch".to_string(), 500)]));
        inventory
            .reserve_source_batch("exec-reserved", "ore-batch", 300)
            .expect("source reservation should succeed");

        let unrelated = InventoryEvent::new(
            1,
            "ore-batch",
            250,
            InventoryEventKind::Consumed,
        )
        .with_event_id("unrelated-consume")
        .with_provenance("other-execution");

        assert!(inventory.append(unrelated).is_err());
        assert_eq!(inventory.state().get("ore-batch"), Some(&500));
        assert!(inventory.events().is_empty());
    }

    #[test]
    fn execution_commit_requires_concrete_source_reservation() {
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
        let mut inventory = InventoryLedger::new(BTreeMap::from([(
            "unreserved-feed".to_string(),
            1_000,
        )]));
        let mut energy = EnergyLedger::new(BTreeMap::from([("bus".to_string(), 5_000)]));

        assert!(
            commit_process_execution(&receipt, &mut budget, &mut inventory, &mut energy).is_err()
        );
        assert_eq!(inventory.state().get("unreserved-feed"), Some(&1_000));
        assert!(inventory.events().is_empty());
        assert_eq!(energy.events().len(), 0);
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
        let receipt = process
            .authorize_execution("exec-004", 10, 20, "power-bus-1", run, &mut budget)
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
        let receipt_a = process
            .authorize_execution("exec-a", 100, 200, "bus", run_a, &mut budget)
            .expect("first execution should authorize");
        let receipt_b = process
            .authorize_execution("exec-b", 100, 200, "bus", run_b, &mut budget)
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
        altered.first_inventory_sequence = 11;
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
        );
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
        );
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
        let claim = ResourceClaim::new("", 5_000, EvidenceGrade::InSituMeasured, 950_000);
        assert!(
            claim
                .certify_for_inventory(EvidenceGrade::InSituMeasured, 900_000)
                .is_err()
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
    fn process_efficiency_is_deterministic() {
        let focused = ProcessEfficiency::new(10, 100, 100);
        let diffuse = ProcessEfficiency::new(12, 200, 200);

        assert!(focused.better_than(diffuse));
        assert!(!diffuse.better_than(focused));
    }

    #[test]
    fn pareto_frontier_prefers_dependency_closure_without_single_score() {
        let dependency_remover =
            BootstrapCandidate::new("close_electronics", 30, 30, 100, 500, 20, 50_000);
        let throughput =
            BootstrapCandidate::new("increase_bulk_output", 0, 0, 100, 500, 20, 50_000);
        let frontier = pareto_frontier(&[throughput, dependency_remover]);

        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].id, "close_electronics");
    }

    #[test]
    fn pareto_frontier_preserves_real_tradeoffs() {
        let low_energy = BootstrapCandidate::new("low_energy", 10, 5, 100, 100, 30, 100_000);
        let low_mass = BootstrapCandidate::new("low_mass", 10, 5, 50, 200, 30, 100_000);
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
        let dominated = BootstrapCandidate::new("same_id", 1, 1, 100, 100, 30, 100_000);
        let stronger = BootstrapCandidate::new("same_id", 2, 2, 90, 90, 20, 90_000);
        let frontier = pareto_frontier(&[dominated, stronger]);

        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].critical_weight_closed_gain, 2);
    }

    #[test]
    fn pareto_frontier_is_input_order_independent() {
        let a = BootstrapCandidate::new("a", 10, 5, 100, 100, 30, 100_000);
        let b = BootstrapCandidate::new("b", 20, 5, 100, 100, 30, 100_000);
        let c = BootstrapCandidate::new("c", 5, 10, 90, 110, 20, 80_000);

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
        let receipt = process
            .authorize_execution("exec-replay", 70, 71, "bus", run, &mut budget)
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
    fn ratio_ppm_is_bounded_and_deterministic() {
        assert_eq!(ratio_ppm(1, 0), 0);
        assert_eq!(ratio_ppm(1, 2), 500_000);
        assert_eq!(ratio_ppm(2, 1), 1_000_000);
    }
}
