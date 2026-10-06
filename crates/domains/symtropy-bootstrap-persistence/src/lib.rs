// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Journal-first durable execution lifecycle integration for the bootstrap kernel.
//!
//! The bootstrap kernel remains storage-independent. This adapter makes the
//! verified persistence journal the durable authority for Pending/Committed/Aborted
//! execution records and treats in-memory kernel state as a checked projection.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
};

use symtropy_bootstrap::{
    abort_pending_execution, abort_process_execution, commit_process_execution,
    combine_execution_state_commitments, resume_pending_execution,
    EnergyLedger, ExecutionStateAnchor, ExecutableProcessExecutionReceipt, ExecutionBudget,
    InventoryLedger, ProcessExecutionReceipt, ProcessRun, ProductionProcess,
};
use symtropy_game_state::{EventChain, EventEnvelope};
use symtropy_persistence::{JournalLoad, PersistenceError, SaveStore};

pub const EXECUTION_LIFECYCLE_SCHEMA_VERSION: u32 = 1;
pub const EXECUTION_EVENT_KIND: &str = "symtropy.bootstrap.execution";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurableExecutionState {
    Pending,
    Committed,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedExecutionAnchor {
    pub domain: String,
    pub frontier: String,
    pub state_commitment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedProcessRun {
    pub process_id: String,
    pub input_material: String,
    pub input_batch_id: String,
    pub feed_mass_g: u64,
    pub output_mass_g: BTreeMap<String, u64>,
    pub waste_mass_g: u64,
    pub energy_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedExecutionReceipt {
    pub execution_id: String,
    pub process_id: String,
    pub input_batch_id: String,
    pub waste_stream: String,
    pub first_inventory_sequence: u64,
    pub energy_sequence: u64,
    pub energy_node_id: String,
    pub state_anchor: PersistedExecutionAnchor,
    pub run: PersistedProcessRun,
}

impl PersistedExecutionReceipt {
    #[must_use]
    pub fn from_receipt(receipt: &ProcessExecutionReceipt) -> Self {
        Self {
            execution_id: receipt.execution_id().to_string(),
            process_id: receipt.process_id().to_string(),
            input_batch_id: receipt.input_batch_id().to_string(),
            waste_stream: receipt.waste_stream().to_string(),
            first_inventory_sequence: receipt.first_inventory_sequence(),
            energy_sequence: receipt.energy_sequence(),
            energy_node_id: receipt.energy_node_id().to_string(),
            state_anchor: PersistedExecutionAnchor {
                domain: receipt.state_anchor().domain().to_string(),
                frontier: receipt.state_anchor().frontier().to_string(),
                state_commitment: receipt.state_anchor().state_commitment().to_string(),
            },
            run: PersistedProcessRun {
                process_id: receipt.run().process_id.clone(),
                input_material: receipt.run().input_material.clone(),
                input_batch_id: receipt.run().input_batch_id.clone(),
                feed_mass_g: receipt.run().feed_mass_g,
                output_mass_g: receipt.run().output_mass_g.clone(),
                waste_mass_g: receipt.run().waste_mass_g,
                energy_units: receipt.run().energy_units,
            },
        }
    }

    pub fn to_receipt(&self) -> Result<ProcessExecutionReceipt, AdapterError> {
        let anchor = ExecutionStateAnchor::new(
            self.state_anchor.domain.clone(),
            self.state_anchor.frontier.clone(),
        )
        .map_err(AdapterError::Invalid)?
        .with_state_commitment(self.state_anchor.state_commitment.clone())
        .map_err(AdapterError::Invalid)?;

        let run = ProcessRun::new(
            self.run.process_id.clone(),
            self.run.input_material.clone(),
            self.run.input_batch_id.clone(),
            self.run.feed_mass_g,
            self.run.output_mass_g.clone(),
            self.run.waste_mass_g,
            self.run.energy_units,
        );

        ProcessExecutionReceipt::from_persisted_parts(
            self.execution_id.clone(),
            self.process_id.clone(),
            self.input_batch_id.clone(),
            self.waste_stream.clone(),
            self.first_inventory_sequence,
            self.energy_sequence,
            self.energy_node_id.clone(),
            anchor,
            run,
        )
        .map_err(AdapterError::Invalid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionLifecycleEvent {
    pub schema_version: u32,
    pub execution_id: String,
    pub state: DurableExecutionState,
    pub receipt: PersistedExecutionReceipt,
    pub receipt_commitment: String,
    pub process_definition_commitment: String,
    pub pending_event_id: Option<String>,
    pub pending_event_hash: Option<String>,
    pub pre_budget_commitment: String,
    pub pre_inventory_commitment: String,
    pub pre_energy_commitment: String,
    pub post_budget_commitment: String,
    pub post_inventory_commitment: String,
    pub post_energy_commitment: String,
    pub causal_inventory_event_ids: Vec<String>,
    pub causal_energy_event_id: Option<String>,
}

impl ExecutionLifecycleEvent {
    fn pending(
        receipt: &ProcessExecutionReceipt,
        process_definition_commitment: String,
        pre_budget_commitment: String,
        pre_inventory_commitment: String,
        pre_energy_commitment: String,
        post_budget_commitment: String,
        post_inventory_commitment: String,
        post_energy_commitment: String,
    ) -> Self {
        Self {
            schema_version: EXECUTION_LIFECYCLE_SCHEMA_VERSION,
            execution_id: receipt.execution_id().to_string(),
            state: DurableExecutionState::Pending,
            receipt: PersistedExecutionReceipt::from_receipt(receipt),
            receipt_commitment: receipt.commitment(),
            process_definition_commitment,
            pending_event_id: None,
            pending_event_hash: None,
            pre_budget_commitment,
            pre_inventory_commitment,
            pre_energy_commitment,
            post_budget_commitment,
            post_inventory_commitment,
            post_energy_commitment,
            causal_inventory_event_ids: Vec::new(),
            causal_energy_event_id: None,
        }
    }

    fn terminal(
        state: DurableExecutionState,
        receipt: &ProcessExecutionReceipt,
        process_definition_commitment: String,
        pending_event_id: String,
        pending_event_hash: String,
        pre_budget_commitment: String,
        pre_inventory_commitment: String,
        pre_energy_commitment: String,
        post_budget_commitment: String,
        post_inventory_commitment: String,
        post_energy_commitment: String,
    ) -> Self {
        let committed = matches!(state, DurableExecutionState::Committed);
        Self {
            schema_version: EXECUTION_LIFECYCLE_SCHEMA_VERSION,
            execution_id: receipt.execution_id().to_string(),
            state,
            receipt: PersistedExecutionReceipt::from_receipt(receipt),
            receipt_commitment: receipt.commitment(),
            process_definition_commitment,
            pending_event_id: Some(pending_event_id),
            pending_event_hash: Some(pending_event_hash),
            pre_budget_commitment,
            pre_inventory_commitment,
            pre_energy_commitment,
            post_budget_commitment,
            post_inventory_commitment,
            post_energy_commitment,
            causal_inventory_event_ids: if committed {
                receipt.inventory_event_ids()
            } else {
                Vec::new()
            },
            causal_energy_event_id: committed.then(|| receipt.energy_event_id()),
        }
    }

    fn validate_basic(&self) -> Result<ProcessExecutionReceipt, AdapterError> {
        if self.schema_version != EXECUTION_LIFECYCLE_SCHEMA_VERSION {
            return Err(AdapterError::Invalid(format!(
                "unsupported execution lifecycle schema {}",
                self.schema_version
            )));
        }
        if self.execution_id.is_empty() || self.execution_id != self.receipt.execution_id {
            return Err(AdapterError::Invalid(
                "lifecycle execution identity does not match receipt".to_string(),
            ));
        }

        for (name, value) in [
            ("receipt commitment", &self.receipt_commitment),
            (
                "process definition commitment",
                &self.process_definition_commitment,
            ),
            ("pre budget commitment", &self.pre_budget_commitment),
            ("pre inventory commitment", &self.pre_inventory_commitment),
            ("pre energy commitment", &self.pre_energy_commitment),
            ("post budget commitment", &self.post_budget_commitment),
            ("post inventory commitment", &self.post_inventory_commitment),
            ("post energy commitment", &self.post_energy_commitment),
        ] {
            if !is_sha256_hex(value) {
                return Err(AdapterError::Invalid(format!(
                    "{name} must be a lowercase SHA-256 commitment"
                )));
            }
        }

        let receipt = self.receipt.to_receipt()?;
        if !receipt.state_anchor().is_state_bound() {
            return Err(AdapterError::Invalid(
                "durable lifecycle receipt cannot use an unbound execution anchor".to_string(),
            ));
        }
        if receipt.commitment() != self.receipt_commitment {
            return Err(AdapterError::Invalid(
                "durable lifecycle receipt commitment mismatch".to_string(),
            ));
        }

        match self.state {
            DurableExecutionState::Pending => {
                if self.pending_event_id.is_some()
                    || self.pending_event_hash.is_some()
                    || !self.causal_inventory_event_ids.is_empty()
                    || self.causal_energy_event_id.is_some()
                {
                    return Err(AdapterError::Invalid(
                        "Pending lifecycle event cannot carry terminal-only links".to_string(),
                    ));
                }
            }
            DurableExecutionState::Committed => {
                let pending_id = self.pending_event_id.as_deref().ok_or_else(|| {
                    AdapterError::Invalid(
                        "Committed lifecycle event is missing Pending identity".to_string(),
                    )
                })?;
                let pending_hash = self.pending_event_hash.as_deref().ok_or_else(|| {
                    AdapterError::Invalid(
                        "Committed lifecycle event is missing Pending hash".to_string(),
                    )
                })?;
                if pending_id.is_empty() || !is_sha256_hex(pending_hash) {
                    return Err(AdapterError::Invalid(
                        "Committed lifecycle event has invalid Pending linkage".to_string(),
                    ));
                }
                if self.causal_inventory_event_ids != receipt.inventory_event_ids()
                    || self.causal_energy_event_id.as_deref()
                        != Some(receipt.energy_event_id().as_str())
                {
                    return Err(AdapterError::Invalid(
                        "Committed lifecycle event causal event identities do not match receipt"
                            .to_string(),
                    ));
                }
            }
            DurableExecutionState::Aborted => {
                if self.pending_event_id.as_deref().is_none_or(str::is_empty)
                    || self
                        .pending_event_hash
                        .as_deref()
                        .is_none_or(|hash| !is_sha256_hex(hash))
                    || !self.causal_inventory_event_ids.is_empty()
                    || self.causal_energy_event_id.is_some()
                {
                    return Err(AdapterError::Invalid(
                        "Aborted lifecycle event has invalid terminal linkage".to_string(),
                    ));
                }
            }
        }
        Ok(receipt)
    }
}

#[derive(Debug)]
pub enum AdapterError {
    Persistence(PersistenceError),
    Invalid(String),
    MissingExecution(String),
    UnexpectedState(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Persistence(error) => write!(f, "persistence error: {error}"),
            Self::Invalid(error) => write!(f, "invalid durable execution record: {error}"),
            Self::MissingExecution(id) => {
                write!(f, "execution is absent from durable journal: {id}")
            }
            Self::UnexpectedState(error) => {
                write!(f, "durable recovery state mismatch: {error}")
            }
        }
    }
}

impl Error for AdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Persistence(error) => Some(error),
            Self::Invalid(_) | Self::MissingExecution(_) | Self::UnexpectedState(_) => None,
        }
    }
}

impl From<PersistenceError> for AdapterError {
    fn from(error: PersistenceError) -> Self {
        Self::Persistence(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    Committed,
    Aborted,
}

#[derive(Debug, Clone)]
pub struct DurableExecutionAdapter {
    store: SaveStore,
    journal_namespace: String,
    anchor_domain: String,
    seed: u64,
}

impl DurableExecutionAdapter {
    pub fn open(
        store: SaveStore,
        journal_namespace: impl Into<String>,
        anchor_domain: impl Into<String>,
        seed: u64,
    ) -> Result<Self, AdapterError> {
        let journal_namespace = journal_namespace.into();
        let anchor_domain = anchor_domain.into();

        if journal_namespace.is_empty() || journal_namespace.len() > 96 {
            return Err(AdapterError::Invalid(
                "execution journal namespace must be non-empty and <= 96 bytes".to_string(),
            ));
        }
        ExecutionStateAnchor::new(anchor_domain.clone(), "GENESIS")
            .map_err(AdapterError::Invalid)?;

        Ok(Self {
            store,
            journal_namespace,
            anchor_domain,
            seed,
        })
    }

    #[must_use]
    pub fn store(&self) -> &SaveStore {
        &self.store
    }

    pub fn load(&self) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        Ok(self
            .store
            .load_journal(self.journal_namespace.clone(), self.seed)?)
    }

    fn load_verified(&self) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        let loaded = self.load_verified()?;
        for event in loaded.chain.events() {
            if event.payload.receipt.state_anchor().domain() != self.anchor_domain {
                return Err(AdapterError::Invalid(
                    "durable execution anchor domain does not match adapter domain"
                        .to_string(),
                ));
            }
        }
        Ok(loaded)
    }

    pub fn validate_journal(
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        chain
            .verify()
            .map_err(|error| AdapterError::Invalid(format!(
                "journal chain verification failed: {error}"
            )))?;

        struct LifecycleSeen {
            event_id: String,
            event_hash: String,
            receipt: PersistedExecutionReceipt,
            receipt_commitment: String,
            process_definition_commitment: String,
            post_budget_commitment: String,
            post_inventory_commitment: String,
            post_energy_commitment: String,
            terminal: bool,
        }

        let mut seen: BTreeMap<String, LifecycleSeen> = BTreeMap::new();

        for event in chain.events() {
            if event.kind != EXECUTION_EVENT_KIND {
                return Err(AdapterError::Invalid(format!(
                    "unexpected execution journal event kind: {}",
                    event.kind
                )));
            }

            let receipt = event.payload.validate_basic()?;

            match event.payload.state {
                DurableExecutionState::Pending => {
                    if event.previous_hash != receipt.state_anchor().frontier() {
                        return Err(AdapterError::Invalid(format!(
                            "Pending event {} is not immediately after its anchored frontier",
                            event.event_id
                        )));
                    }

                    let combined = combine_execution_state_commitments(
                        &event.payload.pre_budget_commitment,
                        &event.payload.pre_inventory_commitment,
                    );
                    if combined != receipt.state_anchor().state_commitment() {
                        return Err(AdapterError::Invalid(
                            "Pending pre-state component commitments do not match receipt anchor"
                                .to_string(),
                        ));
                    }

                    if seen.contains_key(&event.payload.execution_id) {
                        return Err(AdapterError::Invalid(format!(
                            "duplicate Pending lifecycle record: {}",
                            event.payload.execution_id
                        )));
                    }

                    seen.insert(
                        event.payload.execution_id.clone(),
                        LifecycleSeen {
                            event_id: event.event_id.to_string(),
                            event_hash: event.event_hash.clone(),
                            receipt: event.payload.receipt.clone(),
                            receipt_commitment: event.payload.receipt_commitment.clone(),
                            process_definition_commitment: event
                                .payload
                                .process_definition_commitment
                                .clone(),
                            post_budget_commitment: event.payload.post_budget_commitment.clone(),
                            post_inventory_commitment: event
                                .payload
                                .post_inventory_commitment
                                .clone(),
                            post_energy_commitment: event
                                .payload
                                .post_energy_commitment
                                .clone(),
                            terminal: false,
                        },
                    );
                }
                DurableExecutionState::Committed | DurableExecutionState::Aborted => {
                    let prior = seen
                        .get_mut(&event.payload.execution_id)
                        .ok_or_else(|| {
                            AdapterError::Invalid(format!(
                                "terminal event {} has no Pending predecessor",
                                event.event_id
                            ))
                        })?;

                    if prior.terminal {
                        return Err(AdapterError::Invalid(format!(
                            "duplicate terminal lifecycle record: {}",
                            event.payload.execution_id
                        )));
                    }

                    if prior.receipt != event.payload.receipt
                        || prior.receipt_commitment != event.payload.receipt_commitment
                        || prior.process_definition_commitment
                            != event.payload.process_definition_commitment
                    {
                        return Err(AdapterError::Invalid(
                            "terminal lifecycle receipt does not exactly match Pending receipt"
                                .to_string(),
                        ));
                    }

                    if event.payload.pending_event_id.as_deref()
                        != Some(prior.event_id.as_str())
                        || event.payload.pending_event_hash.as_deref()
                            != Some(prior.event_hash.as_str())
                    {
                        return Err(AdapterError::Invalid(
                            "terminal lifecycle does not point to its exact Pending journal record"
                                .to_string(),
                        ));
                    }

                    if event.payload.pre_budget_commitment != prior.post_budget_commitment
                        || event.payload.pre_inventory_commitment
                            != prior.post_inventory_commitment
                        || event.payload.pre_energy_commitment != prior.post_energy_commitment
                    {
                        return Err(AdapterError::Invalid(
                            "terminal pre-state commitments do not equal Pending post-state commitments"
                                .to_string(),
                        ));
                    }

                    if matches!(event.payload.state, DurableExecutionState::Aborted)
                        && event.payload.post_energy_commitment
                            != event.payload.pre_energy_commitment
                    {
                        return Err(AdapterError::Invalid(
                            "aborted execution cannot change energy state".to_string(),
                        ));
                    }

                    prior.terminal = true;
                }
            }
        }

        let open_pending = seen.values().filter(|record| !record.terminal).count();
        if open_pending > 1 {
            return Err(AdapterError::Invalid(
                "durable execution journal contains multiple open Pending executions; lifecycle is not linearized"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn append_payload(
        &self,
        chain: &mut EventChain<ExecutionLifecycleEvent>,
        simulation_tick: u64,
        payload: ExecutionLifecycleEvent,
    ) -> Result<EventEnvelope<ExecutionLifecycleEvent>, AdapterError> {
        chain
            .append(
                simulation_tick,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                payload,
            )
            .map_err(|error| {
                AdapterError::Invalid(format!("cannot append lifecycle event: {error}"))
            })?;

        let event = chain
            .events()
            .last()
            .cloned()
            .ok_or_else(|| AdapterError::Invalid("journal append produced no event".to_string()))?;

        self.store.append_event(&event)?;
        Ok(event)
    }

    fn pending_event(
        chain: &EventChain<ExecutionLifecycleEvent>,
        execution_id: &str,
    ) -> Result<&EventEnvelope<ExecutionLifecycleEvent>, AdapterError> {
        chain
            .events()
            .iter()
            .find(|event| {
                event.payload.execution_id == execution_id
                    && event.payload.state == DurableExecutionState::Pending
            })
            .ok_or_else(|| AdapterError::MissingExecution(execution_id.to_string()))
    }

    fn pending_record(
        chain: &EventChain<ExecutionLifecycleEvent>,
        execution_id: &str,
    ) -> Result<(String, String, PersistedExecutionReceipt), AdapterError> {
        let event = Self::pending_event(chain, execution_id)?;
        Ok((
            event.event_id.to_string(),
            event.event_hash.clone(),
            event.payload.receipt.clone(),
        ))
    }

    fn ensure_process(
        process: &ProductionProcess,
        receipt: &ProcessExecutionReceipt,
    ) -> Result<(), AdapterError> {
        process.validate_run(receipt.run()).map_err(AdapterError::Invalid)
    }

    fn ensure_process_definition(
        process: &ProductionProcess,
        expected_commitment: &str,
    ) -> Result<(), AdapterError> {
        if process.commitment() != expected_commitment {
            return Err(AdapterError::Invalid(
                "process definition commitment does not match durable execution record"
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn ensure_live_matches_latest(
        chain: &EventChain<ExecutionLifecycleEvent>,
        budget: &ExecutionBudget,
        inventory: &InventoryLedger,
        energy: &EnergyLedger,
    ) -> Result<(), AdapterError> {
        let Some(last) = chain.events().last() else {
            return Ok(());
        };

        if budget.state_commitment() != last.payload.post_budget_commitment
            || inventory.state_commitment() != last.payload.post_inventory_commitment
            || energy.state_commitment() != last.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "live projection does not match the durable journal head projection".to_string(),
            ));
        }

        Ok(())
    }

    fn ensure_only_one_pending(
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        if chain.events().iter().any(|event| {
            event.payload.state == DurableExecutionState::Pending
                && !chain.events().iter().any(|terminal| {
                    terminal.payload.execution_id == event.payload.execution_id
                        && matches!(
                            terminal.payload.state,
                            DurableExecutionState::Committed | DurableExecutionState::Aborted
                        )
                })
        }) {
            return Err(AdapterError::UnexpectedState(
                "durable execution adapter permits only one open Pending lifecycle record"
                    .to_string(),
            ));
        }
        Ok(())
    }

    pub fn authorize_pending(
        &self,
        process: &ProductionProcess,
        execution_id: impl Into<String>,
        simulation_tick: u64,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &EnergyLedger,
    ) -> Result<ProcessExecutionReceipt, AdapterError> {
        let execution_id = execution_id.into();
        let loaded = self.load()?;
        Self::validate_journal(&loaded.chain)?;
        Self::ensure_only_one_pending(&loaded.chain)?;
        Self::ensure_live_matches_latest(&loaded.chain, budget, inventory, energy)?;

        if loaded
            .chain
            .events()
            .iter()
            .any(|event| event.payload.execution_id == execution_id)
        {
            return Err(AdapterError::Invalid(format!(
                "execution identity already exists in durable journal: {execution_id}"
            )));
        }

        let pre_budget_commitment = budget.state_commitment();
        let pre_inventory_commitment = inventory.state_commitment();
        let pre_energy_commitment = energy.state_commitment();
        let process_definition_commitment = process.commitment();

        let anchor = ExecutionStateAnchor::for_state(
            self.anchor_domain.clone(),
            loaded.chain.head_hash().to_string(),
            budget,
            inventory,
        )
        .map_err(AdapterError::Invalid)?;

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();

        let receipt = process
            .authorize_pending_execution_with_inventory_at_anchor(
                execution_id,
                first_inventory_sequence,
                energy_sequence,
                node_id,
                anchor,
                run,
                &mut staged_budget,
                &mut staged_inventory,
            )
            .map_err(AdapterError::Invalid)?;

        let payload = ExecutionLifecycleEvent::pending(
            &receipt,
            process_definition_commitment,
            pre_budget_commitment,
            pre_inventory_commitment,
            pre_energy_commitment.clone(),
            staged_budget.state_commitment(),
            staged_inventory.state_commitment(),
            pre_energy_commitment,
        );

        let mut chain = loaded.chain;
        self.append_payload(&mut chain, simulation_tick, payload)?;

        *budget = staged_budget;
        *inventory = staged_inventory;
        Ok(receipt)
    }

    pub fn recover_pending(
        &self,
        process: &ProductionProcess,
        execution_id: &str,
        expected_anchor: &ExecutionStateAnchor,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &EnergyLedger,
    ) -> Result<ExecutableProcessExecutionReceipt, AdapterError> {
        let loaded = self.load()?;
        Self::validate_journal(&loaded.chain)?;
        let event = Self::pending_event(&loaded.chain, execution_id)?;
        let receipt = event.payload.receipt.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &event.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &receipt)?;

        if receipt.state_anchor() != expected_anchor {
            return Err(AdapterError::Invalid(
                "recovery anchor does not match persisted receipt".to_string(),
            ));
        }
        if !expected_anchor.is_state_bound() {
            return Err(AdapterError::Invalid(
                "recovery requires a state-bound durable execution anchor".to_string(),
            ));
        }

        let live_post_matches = budget.state_commitment() == event.payload.post_budget_commitment
            && inventory.state_commitment() == event.payload.post_inventory_commitment
            && energy.state_commitment() == event.payload.post_energy_commitment;

        if live_post_matches {
            return resume_pending_execution(execution_id, budget, inventory)
                .map_err(AdapterError::Invalid);
        }

        let live_pre_matches = budget.state_commitment() == event.payload.pre_budget_commitment
            && inventory.state_commitment() == event.payload.pre_inventory_commitment
            && energy.state_commitment() == event.payload.pre_energy_commitment;

        if !live_pre_matches {
            return Err(AdapterError::UnexpectedState(
                "live state matches neither durable pre-Pending nor post-Pending commitments"
                    .to_string(),
            ));
        }

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();

        process
            .restore_pending_execution_with_inventory_at_anchor(
                &receipt,
                expected_anchor,
                &mut staged_budget,
                &mut staged_inventory,
            )
            .map_err(AdapterError::Invalid)?;

        if staged_budget.state_commitment() != event.payload.post_budget_commitment
            || staged_inventory.state_commitment() != event.payload.post_inventory_commitment
            || energy.state_commitment() != event.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "restored Pending execution does not match durable post-state commitments"
                    .to_string(),
            ));
        }

        *budget = staged_budget;
        *inventory = staged_inventory;

        resume_pending_execution(execution_id, budget, inventory)
            .map_err(AdapterError::Invalid)
    }

    pub fn commit(
        &self,
        process: &ProductionProcess,
        receipt: &ExecutableProcessExecutionReceipt,
        simulation_tick: u64,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &mut EnergyLedger,
    ) -> Result<(), AdapterError> {
        let loaded = self.load()?;
        Self::validate_journal(&loaded.chain)?;

        let (pending_event_id, pending_event_hash, persisted) =
            Self::pending_record(&loaded.chain, receipt.execution_id())?;
        let pending_event = Self::pending_event(&loaded.chain, receipt.execution_id())?;
        let pending = persisted.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &pending_event.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &pending)?;

        if receipt.commitment() != pending.commitment() {
            return Err(AdapterError::Invalid(
                "executable receipt does not match durable Pending receipt".to_string(),
            ));
        }

        if budget.state_commitment() != pending_event.payload.post_budget_commitment
            || inventory.state_commitment() != pending_event.payload.post_inventory_commitment
            || energy.state_commitment() != pending_event.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "live projection does not match durable Pending post-state".to_string(),
            ));
        }

        let pre_budget = budget.state_commitment();
        let pre_inventory = inventory.state_commitment();
        let pre_energy = energy.state_commitment();

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();
        let mut staged_energy = energy.clone();

        commit_process_execution(
            receipt,
            &mut staged_budget,
            &mut staged_inventory,
            &mut staged_energy,
        )
        .map_err(AdapterError::Invalid)?;

        let payload = ExecutionLifecycleEvent::terminal(
            DurableExecutionState::Committed,
            &pending,
            pending_event.payload.process_definition_commitment.clone(),
            pending_event_id,
            pending_event_hash,
            pre_budget,
            pre_inventory,
            pre_energy,
            staged_budget.state_commitment(),
            staged_inventory.state_commitment(),
            staged_energy.state_commitment(),
        );

        let mut chain = loaded.chain;
        self.append_payload(&mut chain, simulation_tick, payload)?;

        *budget = staged_budget;
        *inventory = staged_inventory;
        *energy = staged_energy;
        Ok(())
    }

    pub fn abort(
        &self,
        process: &ProductionProcess,
        receipt: &ExecutableProcessExecutionReceipt,
        simulation_tick: u64,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &mut EnergyLedger,
    ) -> Result<(), AdapterError> {
        let loaded = self.load()?;
        Self::validate_journal(&loaded.chain)?;

        let (pending_event_id, pending_event_hash, persisted) =
            Self::pending_record(&loaded.chain, receipt.execution_id())?;
        let pending_event = Self::pending_event(&loaded.chain, receipt.execution_id())?;
        let pending = persisted.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &pending_event.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &pending)?;

        if receipt.commitment() != pending.commitment() {
            return Err(AdapterError::Invalid(
                "executable receipt does not match durable Pending receipt".to_string(),
            ));
        }

        if budget.state_commitment() != pending_event.payload.post_budget_commitment
            || inventory.state_commitment() != pending_event.payload.post_inventory_commitment
            || energy.state_commitment() != pending_event.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "live projection does not match durable Pending post-state".to_string(),
            ));
        }

        let pre_budget = budget.state_commitment();
        let pre_inventory = inventory.state_commitment();
        let pre_energy = energy.state_commitment();

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();

        abort_process_execution(receipt, &mut staged_budget, &mut staged_inventory)
            .map_err(AdapterError::Invalid)?;

        let payload = ExecutionLifecycleEvent::terminal(
            DurableExecutionState::Aborted,
            &pending,
            pending_event.payload.process_definition_commitment.clone(),
            pending_event_id,
            pending_event_hash,
            pre_budget,
            pre_inventory,
            pre_energy,
            staged_budget.state_commitment(),
            staged_inventory.state_commitment(),
            energy.state_commitment(),
        );

        let mut chain = loaded.chain;
        self.append_payload(&mut chain, simulation_tick, payload)?;

        *budget = staged_budget;
        *inventory = staged_inventory;
        Ok(())
    }

    pub fn recover_terminal(
        &self,
        process: &ProductionProcess,
        execution_id: &str,
        expected_anchor: &ExecutionStateAnchor,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &mut EnergyLedger,
    ) -> Result<RecoveryResult, AdapterError> {
        let loaded = self.load()?;
        Self::validate_journal(&loaded.chain)?;

        let terminal = loaded
            .chain
            .events()
            .iter()
            .find(|event| {
                event.payload.execution_id == execution_id
                    && event.payload.state != DurableExecutionState::Pending
            })
            .ok_or_else(|| AdapterError::MissingExecution(execution_id.to_string()))?;

        let receipt = terminal.payload.receipt.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &terminal.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &receipt)?;

        if receipt.state_anchor() != expected_anchor {
            return Err(AdapterError::Invalid(
                "terminal recovery anchor does not match persisted receipt".to_string(),
            ));
        }
        if !expected_anchor.is_state_bound() {
            return Err(AdapterError::Invalid(
                "terminal recovery requires a state-bound durable execution anchor".to_string(),
            ));
        }

        let post_matches = budget.state_commitment() == terminal.payload.post_budget_commitment
            && inventory.state_commitment() == terminal.payload.post_inventory_commitment
            && energy.state_commitment() == terminal.payload.post_energy_commitment;

        if post_matches {
            return Ok(match terminal.payload.state {
                DurableExecutionState::Committed => RecoveryResult::Committed,
                DurableExecutionState::Aborted => RecoveryResult::Aborted,
                DurableExecutionState::Pending => unreachable!(),
            });
        }

        let pre_matches = budget.state_commitment() == terminal.payload.pre_budget_commitment
            && inventory.state_commitment() == terminal.payload.pre_inventory_commitment
            && energy.state_commitment() == terminal.payload.pre_energy_commitment;

        if !pre_matches {
            return Err(AdapterError::UnexpectedState(
                "live state matches neither durable terminal pre-state nor post-state commitments"
                    .to_string(),
            ));
        }

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();
        let mut staged_energy = energy.clone();

        match terminal.payload.state {
            DurableExecutionState::Committed => {
                let executable =
                    resume_pending_execution(execution_id, &staged_budget, &staged_inventory)
                        .map_err(AdapterError::Invalid)?;
                commit_process_execution(
                    &executable,
                    &mut staged_budget,
                    &mut staged_inventory,
                    &mut staged_energy,
                )
                .map_err(AdapterError::Invalid)?;
            }
            DurableExecutionState::Aborted => {
                abort_pending_execution(&receipt, &mut staged_budget, &mut staged_inventory)
                    .map_err(AdapterError::Invalid)?;
            }
            DurableExecutionState::Pending => unreachable!(),
        }

        if staged_budget.state_commitment() != terminal.payload.post_budget_commitment
            || staged_inventory.state_commitment() != terminal.payload.post_inventory_commitment
            || staged_energy.state_commitment() != terminal.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "replayed terminal transition does not match durable post-state commitments"
                    .to_string(),
            ));
        }

        *budget = staged_budget;
        *inventory = staged_inventory;
        *energy = staged_energy;

        Ok(match terminal.payload.state {
            DurableExecutionState::Committed => RecoveryResult::Committed,
            DurableExecutionState::Aborted => RecoveryResult::Aborted,
            DurableExecutionState::Pending => unreachable!(),
        })
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn store(name: &str) -> SaveStore {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        SaveStore::open(std::env::temp_dir().join(format!(
            "symtropy-bootstrap-adapter-{name}-{suffix}"
        )))
        .expect("store")
    }

    fn process_and_run() -> (ProductionProcess, ProcessRun) {
        (
            ProductionProcess::new("electrolysis", "regolith", ["oxygen", "metal"], "slag"),
            ProcessRun::new(
                "electrolysis",
                "regolith",
                "feed",
                1_000,
                BTreeMap::from([("metal".to_string(), 720), ("oxygen".to_string(), 180)]),
                100,
                4_000,
            ),
        )
    }

    fn initial_kernel_state() -> (ExecutionBudget, InventoryLedger, EnergyLedger) {
        (
            ExecutionBudget::new(1_000, 4_000),
            InventoryLedger::new(BTreeMap::from([("feed".to_string(), 1_000)])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 4_000)])),
        )
    }

    #[test]
    fn pending_record_is_write_ahead_of_live_projection() {
        let adapter = DurableExecutionAdapter::open(
            store("pending"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        let pre_budget = budget.state_commitment();
        let pre_inventory = inventory.state_commitment();
        let pre_energy = energy.state_commitment();

        let _receipt = adapter
            .authorize_pending(
                &process,
                "exec-001",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        let journal = adapter.load().expect("journal");
        let event = &journal.chain.events()[0];
        assert_eq!(journal.chain.events().len(), 1);
        assert_eq!(event.previous_hash, "GENESIS");
        assert_eq!(event.payload.receipt_commitment, receipt.commitment());
        assert_eq!(event.payload.pre_budget_commitment, pre_budget);
        assert_eq!(event.payload.pre_inventory_commitment, pre_inventory);
        assert_eq!(event.payload.pre_energy_commitment, pre_energy);
        assert_eq!(
            event.payload.post_budget_commitment,
            budget.state_commitment()
        );
        assert_eq!(
            event.payload.post_inventory_commitment,
            inventory.state_commitment()
        );
        assert_eq!(event.payload.post_energy_commitment, energy.state_commitment());
        assert_eq!(
            budget.execution_state("exec-001"),
            Some(symtropy_bootstrap::ExecutionState::Pending)
        );
        assert!(inventory.events().is_empty());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn semantic_anchor_tampering_is_rejected_by_valid_outer_chain() {
        let adapter = DurableExecutionAdapter::open(
            store("anchor"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        adapter
            .authorize_pending(
                &process,
                "exec-anchor",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        let loaded = adapter.load().expect("journal");
        let original = loaded.chain.events()[0].clone();
        let mut payload = original.payload.clone();
        payload.receipt.state_anchor.frontier = "wrong-frontier".to_string();
        payload.receipt_commitment = payload
            .receipt
            .to_receipt()
            .expect("tampered receipt should reconstruct")
            .commitment();

        let mut bad_chain = EventChain::new("bootstrap", 1);
        bad_chain
            .append(
                1,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                payload,
            )
            .expect("outer chain can hash tampered payload");

        assert!(DurableExecutionAdapter::validate_journal(&bad_chain).is_err());
        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn terminal_pre_state_must_equal_pending_post_state() {
        let adapter = DurableExecutionAdapter::open(
            store("continuity"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        adapter
            .authorize_pending(
                &process,
                "exec-continuity",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        let loaded = adapter.load().expect("journal");
        let pending = loaded.chain.events()[0].clone();
        let persisted = pending.payload.receipt.to_receipt().expect("receipt");
        let terminal_payload = ExecutionLifecycleEvent::terminal(
            DurableExecutionState::Aborted,
            &persisted,
            pending.event_id.to_string(),
            pending.event_hash.clone(),
            ExecutionBudget::new(999, 3_999).state_commitment(),
            pending.payload.post_inventory_commitment.clone(),
            pending.payload.post_energy_commitment.clone(),
            budget.state_commitment(),
            inventory.state_commitment(),
            energy.state_commitment(),
        );

        let mut bad_chain = loaded.chain.clone();
        bad_chain
            .append(
                2,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                terminal_payload,
            )
            .expect("outer chain can hash terminal mismatch");

        assert!(DurableExecutionAdapter::validate_journal(&bad_chain).is_err());
        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn recover_pending_from_pre_state_rehydrates_without_new_identity() {
        let adapter = DurableExecutionAdapter::open(
            store("recover-pending"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();
        let pre_budget = budget.clone();
        let pre_inventory = inventory.clone();
        let pre_energy = energy.clone();

        let receipt = adapter
            .authorize_pending(
                &process,
                "exec-recover-pending",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        budget = pre_budget;
        inventory = pre_inventory;
        let mut energy = pre_energy;

        let executable = adapter
            .recover_pending(
                &process,
                "exec-recover-pending",
                receipt.state_anchor(),
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending recovery");

        assert_eq!(executable.execution_id(), "exec-recover-pending");
        assert_eq!(
            budget.execution_state("exec-recover-pending"),
            Some(symtropy_bootstrap::ExecutionState::Pending)
        );
        assert!(!inventory.source_reservations.is_empty());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn commit_records_exact_causal_chain_and_post_state() {
        let adapter = DurableExecutionAdapter::open(
            store("commit"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        let receipt = adapter
            .authorize_pending(
                &process,
                "exec-commit",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution(receipt.execution_id(), &budget, &inventory)
            .expect("activation");

        adapter
            .commit(
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let journal = adapter.load().expect("journal");
        assert_eq!(journal.chain.events().len(), 2);
        let pending = &journal.chain.events()[0];
        let terminal = &journal.chain.events()[1];
        assert_eq!(terminal.payload.state, DurableExecutionState::Committed);
        assert_eq!(
            terminal.payload.pending_event_id.as_deref(),
            Some(pending.event_id.to_string().as_str())
        );
        assert_eq!(
            terminal.payload.pending_event_hash.as_deref(),
            Some(pending.event_hash.as_str())
        );
        assert_eq!(
            terminal.payload.causal_inventory_event_ids,
            receipt.inventory_event_ids()
        );
        assert_eq!(
            terminal.payload.causal_energy_event_id.as_deref(),
            Some(receipt.energy_event_id().as_str())
        );
        assert_eq!(
            terminal.payload.pre_budget_commitment,
            pending.payload.post_budget_commitment
        );
        assert_eq!(
            terminal.payload.pre_inventory_commitment,
            pending.payload.post_inventory_commitment
        );
        assert_eq!(
            terminal.payload.pre_energy_commitment,
            pending.payload.post_energy_commitment
        );
        assert_eq!(
            terminal.payload.post_budget_commitment,
            budget.state_commitment()
        );
        assert_eq!(
            terminal.payload.post_inventory_commitment,
            inventory.state_commitment()
        );
        assert_eq!(
            terminal.payload.post_energy_commitment,
            energy.state_commitment()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn recover_terminal_from_pre_state_replays_exact_terminal_result() {
        let adapter = DurableExecutionAdapter::open(
            store("recover-terminal"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        let receipt = adapter
            .authorize_pending(
                &process,
                "exec-recover-terminal",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution(receipt.execution_id(), &budget, &inventory)
            .expect("activation");

        let terminal_pre_budget = budget.clone();
        let terminal_pre_inventory = inventory.clone();
        let terminal_pre_energy = energy.clone();

        adapter
            .commit(
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let durable_post_budget = budget.state_commitment();
        let durable_post_inventory = inventory.state_commitment();
        let durable_post_energy = energy.state_commitment();

        budget = terminal_pre_budget;
        inventory = terminal_pre_inventory;
        energy = terminal_pre_energy;

        let result = adapter
            .recover_terminal(
                &process,
                "exec-recover-terminal",
                receipt.state_anchor(),
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("terminal recovery");

        assert_eq!(result, RecoveryResult::Committed);
        assert_eq!(budget.state_commitment(), durable_post_budget);
        assert_eq!(inventory.state_commitment(), durable_post_inventory);
        assert_eq!(energy.state_commitment(), durable_post_energy);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn duplicate_terminal_is_rejected() {
        let adapter = DurableExecutionAdapter::open(
            store("duplicate-terminal"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        let receipt = adapter
            .authorize_pending(
                &process,
                "exec-duplicate-terminal",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution(receipt.execution_id(), &budget, &inventory)
            .expect("activation");
        adapter
            .commit(
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let loaded = adapter.load().expect("journal");
        let terminal_payload = loaded.chain.events()[1].payload.clone();
        let mut bad_chain = loaded.chain.clone();
        bad_chain
            .append(
                3,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                terminal_payload,
            )
            .expect("outer chain can encode duplicate terminal");

        assert!(DurableExecutionAdapter::validate_journal(&bad_chain).is_err());
        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn authorizing_second_pending_execution_is_rejected() {
        let adapter = DurableExecutionAdapter::open(
            store("single-flight"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        adapter
            .authorize_pending(
                &process,
                "exec-first",
                1,
                10,
                20,
                "bus",
                run.clone(),
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("first pending authorization");

        assert!(
            adapter
                .authorize_pending(
                    &process,
                    "exec-second",
                    2,
                    20,
                    30,
                    "bus",
                    run,
                    &mut budget,
                    &mut inventory,
                    &energy,
                )
                .is_err()
        );

        assert_eq!(
            budget.execution_state("exec-first"),
            Some(symtropy_bootstrap::ExecutionState::Pending)
        );
        assert!(budget.execution_state("exec-second").is_none());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn changed_process_definition_is_rejected_during_recovery() {
        let adapter = DurableExecutionAdapter::open(
            store("process-definition"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let changed_process =
            ProductionProcess::new("electrolysis", "regolith", ["metal", "oxygen"], "slag");
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        let receipt = adapter
            .authorize_pending(
                &process,
                "exec-definition",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        assert_ne!(process.commitment(), changed_process.commitment());
        assert!(
            adapter
                .recover_pending(
                    &changed_process,
                    "exec-definition",
                    receipt.state_anchor(),
                    &mut budget,
                    &mut inventory,
                    &energy,
                )
                .is_err()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn live_projection_mismatch_is_rejected_before_new_pending_authorization() {
        let adapter = DurableExecutionAdapter::open(
            store("projection-mismatch"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter");
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        let receipt = adapter
            .authorize_pending(
                &process,
                "exec-projection",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution("exec-projection", &budget, &inventory)
            .expect("activation");

        adapter
            .commit(
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        energy
            .append(
                symtropy_bootstrap::EnergyEvent::new(
                    99,
                    "bus",
                    1,
                    symtropy_bootstrap::EnergyEventKind::Generated,
                )
                .with_event_id("unrecorded-energy")
                .with_provenance("projection-test"),
            )
            .expect("mutate live-only energy projection");

        assert!(
            adapter
                .authorize_pending(
                    &process,
                    "exec-fork",
                    3,
                    20,
                    30,
                    "bus",
                    process_and_run().1,
                    &mut budget,
                    &mut inventory,
                    &mut energy,
                )
                .is_err()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }
}
