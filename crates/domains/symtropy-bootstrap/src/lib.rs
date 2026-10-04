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
}

impl ClosureReport {
    #[must_use]
    pub const fn fully_closed(&self) -> bool {
        self.weighted_critical_closed == self.weighted_critical_total
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
}

impl DependencyGraph {
    #[must_use]
    pub fn new(
        capabilities: impl IntoIterator<Item = Capability>,
        dependencies: impl IntoIterator<Item = Dependency>,
    ) -> Self {
        Self {
            capabilities: capabilities
                .into_iter()
                .map(|capability| (capability.id.clone(), capability))
                .collect(),
            dependencies: dependencies
                .into_iter()
                .map(|dependency| (dependency.id.clone(), dependency))
                .collect(),
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

        let weighted_critical_total = self
            .capabilities
            .values()
            .map(|capability| capability.critical_weight)
            .sum::<u64>();

        let weighted_critical_closed = assessments
            .iter()
            .filter(|assessment| assessment.closed)
            .filter_map(|assessment| self.capabilities.get(&assessment.id))
            .map(|capability| capability.critical_weight)
            .sum::<u64>();

        ClosureReport {
            assessments,
            weighted_critical_closed,
            weighted_critical_total,
            critical_closure_ppm: ratio_ppm(weighted_critical_closed, weighted_critical_total),
            mass_closure_ppm: ratio_ppm(local_mass_g, local_mass_g.saturating_add(imported_mass_g)),
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
    let closed = report
        .assessments
        .iter()
        .filter(|assessment| assessment.closed)
        .map(|assessment| assessment.id.as_str())
        .collect::<BTreeSet<_>>();

    let mut ordered = requirements.to_vec();
    ordered.sort_by_key(|requirement| requirement.stage);

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

        let declared = self
            .output_streams
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();

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

    /// Materialize a process execution into causal inventory events.
    ///
    /// The feed batch is consumed once; declared product streams are produced
    /// under deterministic batch IDs derived from process identity and sequence.
    /// Waste is intentionally not added to inventory.
    pub fn inventory_events(
        &self,
        first_sequence: u64,
        run: &ProcessRun,
        available_feed_mass_g: u64,
        available_energy_units: u64,
    ) -> Result<Vec<InventoryEvent>, String> {
        self.validate_run_against_budget(run, available_feed_mass_g, available_energy_units)?;

        let cause = format!("process:{}:{first_sequence}", self.id);
        let mut events = Vec::with_capacity(run.output_mass_g.len() + 1);
        events.push(
            InventoryEvent::new(
                first_sequence,
                run.input_batch_id.clone(),
                run.feed_mass_g,
                InventoryEventKind::Consumed,
            )
            .with_provenance(cause.clone()),
        );

        for (offset, (stream, mass)) in run.output_mass_g.iter().enumerate() {
            let sequence = first_sequence
                .checked_add(offset as u64 + 1)
                .ok_or_else(|| "process inventory sequence overflow".to_string())?;
            let batch_id = format!("{}:{first_sequence}:{stream}", self.id);
            events.push(
                InventoryEvent::new(sequence, batch_id, *mass, InventoryEventKind::Produced)
                    .with_provenance(cause.clone()),
            );
        }

        Ok(events)
    }

    /// Create the causal energy-consumption event for a process execution.
    pub fn energy_event(
        &self,
        sequence: u64,
        node_id: impl Into<String>,
        run: &ProcessRun,
        available_feed_mass_g: u64,
        available_energy_units: u64,
    ) -> Result<EnergyEvent, String> {
        self.validate_run_against_budget(run, available_feed_mass_g, available_energy_units)?;

        Ok(EnergyEvent::new(
            sequence,
            node_id,
            run.energy_units,
            EnergyEventKind::Consumed,
        )
        .with_provenance(format!("process:{}:{sequence}", self.id)))
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
            node_id: node_id.into(),
            energy_units,
            kind,
            provenance_id: None,
        }
    }

    /// Attach a stable causal provenance identifier to an energy event.
    #[must_use]
    pub fn with_provenance(mut self, provenance_id: impl Into<String>) -> Self {
        self.provenance_id = Some(provenance_id.into());
        self
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

    let mut energy = initial.clone();

    for event in ordered {
        if event
            .provenance_id
            .as_deref()
            .is_none_or(str::is_empty)
        {
            return Err("energy event requires non-empty provenance".to_string());
        }

        let balance = energy.entry(event.node_id).or_insert(0);

        match event.kind {
            EnergyEventKind::Generated | EnergyEventKind::Recovered => {
                *balance = balance
                    .checked_add(event.energy_units)
                    .ok_or_else(|| "energy overflow".to_string())?;
            }
            EnergyEventKind::Consumed => {
                if *balance < event.energy_units {
                    return Err("energy underflow".to_string());
                }
                *balance -= event.energy_units;
            }
        }
    }

    Ok(energy)
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

        Ok(CertifiedResource {
            claim_id: self.id.clone(),
            mass_g: self.mass_g,
        })
    }
}

/// An explicitly certified resource quantity that may enter inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertifiedResource {
    pub claim_id: String,
    pub mass_g: u64,
}

/// Causal inventory event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryEventKind {
    Produced,
    Consumed,
    Recycled,
}

/// One append-only inventory event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEvent {
    pub sequence: u64,
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
            batch_id: batch_id.into(),
            mass_g,
            kind,
            provenance_id: None,
        }
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
        .with_provenance(resource.claim_id.clone())
    }
}

/// Replay inventory from an initial balance and append-only causal events.
///
/// Events are sorted by sequence so input ordering cannot change the result.
/// Duplicate sequence numbers and underflow are rejected.
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

    let mut inventory = initial.clone();

    for event in ordered {
        if event
            .provenance_id
            .as_deref()
            .is_none_or(str::is_empty)
        {
            return Err("inventory event requires non-empty provenance".to_string());
        }

        let balance = inventory.entry(event.batch_id).or_insert(0);

        match event.kind {
            InventoryEventKind::Produced | InventoryEventKind::Recycled => {
                *balance = balance
                    .checked_add(event.mass_g)
                    .ok_or_else(|| "inventory overflow".to_string())?;
            }
            InventoryEventKind::Consumed => {
                if *balance < event.mass_g {
                    return Err("inventory underflow".to_string());
                }
                *balance -= event.mass_g;
            }
        }
    }

    Ok(inventory)
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
    fn process_events_reject_unbudgeted_execution() {
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

        assert!(process.energy_event(40, "bus", &run, 999, 4_000).is_err());
        assert!(process.inventory_events(40, &run, 1_000, 3_999).is_err());
    }

    #[test]
    fn process_execution_generates_causal_energy_event() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-005",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        let event = process
            .energy_event(30, "power-bus-1", &run, 1_000, 4_000)
            .expect("valid process should emit a causal energy event");
        assert_eq!(event.kind, EnergyEventKind::Consumed);
        assert_eq!(event.node_id, "power-bus-1");
        assert_eq!(event.energy_units, 4_000);
        assert_eq!(
            event.provenance_id.as_deref(),
            Some("process:regolith_electrolysis:30")
        );

        let initial = BTreeMap::from([("power-bus-1".to_string(), 5_000)]);
        let expected = BTreeMap::from([("power-bus-1".to_string(), 1_000)]);
        verify_energy_conservation(&initial, &[event], &expected)
            .expect("energy ledger must replay exactly");
    }

    #[test]
    fn energy_replay_rejects_unfunded_and_duplicate_history() {
        let initial = BTreeMap::from([("bus".to_string(), 1_000)]);
        let overdraw = [EnergyEvent::new(1, "bus", 1_001, EnergyEventKind::Consumed)];
        assert!(replay_energy(&initial, &overdraw).is_err());

        let duplicate = [
            EnergyEvent::new(1, "bus", 1, EnergyEventKind::Generated),
            EnergyEvent::new(1, "bus", 1, EnergyEventKind::Generated),
        ];
        assert!(replay_energy(&BTreeMap::new(), &duplicate).is_err());
    }

    #[test]
    fn process_run_rejects_unbudgeted_execution() {
        let process = ProductionProcess::new(
            "regolith_electrolysis",
            "regolith",
            ["oxygen", "metal"],
            "waste",
        );
        let run = ProcessRun::new(
            "regolith_electrolysis",
            "regolith",
            "feed-003",
            1_000,
            BTreeMap::from([("oxygen".to_string(), 180), ("metal".to_string(), 720)]),
            100,
            4_000,
        );

        assert!(
            process
                .validate_run_against_budget(&run, 999, 4_000)
                .is_err()
        );
        assert!(
            process
                .validate_run_against_budget(&run, 1_000, 3_999)
                .is_err()
        );
        process
            .validate_run_against_budget(&run, 1_000, 4_000)
            .expect("fully funded process should validate");
    }

    #[test]
    fn process_execution_generates_causal_inventory_events() {
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

        let events = process
            .inventory_events(10, &run, 1_000, 4_000)
            .expect("valid process should materialize into events");

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].kind, InventoryEventKind::Consumed);
        assert_eq!(events[0].batch_id, "feed-004");
        assert_eq!(
            events[0].provenance_id.as_deref(),
            Some("process:regolith_electrolysis:10")
        );
        assert_eq!(events[1].sequence, 11);
        assert_eq!(events[2].sequence, 12);
        assert!(events[1].batch_id.ends_with(":oxygen"));
        assert!(events[2].batch_id.ends_with(":metal"));
        assert!(events.iter().all(|event| event.provenance_id.is_some()));

        let final_inventory = replay_inventory(
            &BTreeMap::from([("feed-004".to_string(), 1_000)]),
            &events,
        )
        .expect("generated events must replay");
        assert_eq!(final_inventory.get("feed-004"), Some(&0));
        assert_eq!(
            final_inventory.get("regolith_electrolysis:10:oxygen"),
            Some(&180)
        );
        assert_eq!(
            final_inventory.get("regolith_electrolysis:10:metal"),
            Some(&720)
        );
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
        let event = InventoryEvent::from_certified_resource(20, "ice-batch-001", &certified);

        assert_eq!(event.kind, InventoryEventKind::Produced);
        assert_eq!(event.mass_g, 5_000);
        assert_eq!(event.provenance_id.as_deref(), Some("mercury-polar-ice"));
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
        let initial = BTreeMap::from([("steel".to_string(), 100)]);
        let events = [
            InventoryEvent::new(1, "steel", 25, InventoryEventKind::Produced)
                .with_provenance("forge-run-1"),
            InventoryEvent::new(2, "steel", 60, InventoryEventKind::Consumed)
                .with_provenance("forge-run-2"),
            InventoryEvent::new(3, "steel", 10, InventoryEventKind::Recycled)
                .with_provenance("recycle-run-1"),
        ];
        let expected = BTreeMap::from([("steel".to_string(), 75)]);

        verify_inventory_conservation(&initial, &events, &expected)
            .expect("causal inventory replay must match");

        let reordered = [events[2].clone(), events[0].clone(), events[1].clone()];
        assert_eq!(
            replay_inventory(&initial, &reordered).expect("replay must succeed"),
            expected
        );
    }

    #[test]
    fn inventory_replay_rejects_bad_history() {
        let initial = BTreeMap::from([("steel".to_string(), 1)]);

        let overdraw = [InventoryEvent::new(
            1,
            "steel",
            2,
            InventoryEventKind::Consumed,
        )];
        assert!(replay_inventory(&initial, &overdraw).is_err());

        let duplicate = [
            InventoryEvent::new(1, "a", 1, InventoryEventKind::Produced)
                .with_provenance("a"),
            InventoryEvent::new(1, "b", 1, InventoryEventKind::Produced)
                .with_provenance("b"),
        ];
        assert!(replay_inventory(&BTreeMap::new(), &duplicate).is_err());
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

        let energy = [EnergyEvent::new(
            1,
            "bus",
            1,
            EnergyEventKind::Generated,
        )];
        assert!(replay_energy(&BTreeMap::new(), &energy).is_err());
    }

    #[test]
    fn ratio_ppm_is_bounded_and_deterministic() {
        assert_eq!(ratio_ppm(1, 0), 0);
        assert_eq!(ratio_ppm(1, 2), 500_000);
        assert_eq!(ratio_ppm(2, 1), 1_000_000);
    }
}
