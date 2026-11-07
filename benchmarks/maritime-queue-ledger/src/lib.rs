#![forbid(unsafe_code)]

//! Event-time-correct, evidence-bounded port queue and cargo stock ledgers.
//!
//! This crate audits deterministic accounting snapshots. It does not ingest AIS,
//! authenticate publishers, infer missing events, simulate port physics, or
//! optimize/authorize vessel or terminal operations.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickWindow {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl TickWindow {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventClass {
    Planned,
    Requested,
    Estimated,
    Actual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightsStatus {
    ReviewedAllowed,
    Unknown,
    Restricted,
}

/// A reviewed source receipt. Digest syntax is checked; raw bytes are not
/// fetched or hashed by this crate. Source authenticity and rights truth remain
/// external review responsibilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEvidence {
    pub evidence_id: String,
    pub source_id: String,
    pub source_revision: String,
    pub raw_sha256: String,
    pub rights_reference: String,
    pub rights_status: RightsStatus,
    pub claim_ceiling: String,
}

impl SourceEvidence {
    fn validate(&self) -> Result<(), LedgerError> {
        if self.evidence_id.trim().is_empty()
            || self.source_id.trim().is_empty()
            || self.source_revision.trim().is_empty()
            || self.rights_reference.trim().is_empty()
            || self.claim_ceiling.trim().is_empty()
            || !is_sha256(&self.raw_sha256)
        {
            return Err(LedgerError::InvalidSourceEvidence(
                self.evidence_id.clone(),
            ));
        }
        match self.rights_status {
            RightsStatus::ReviewedAllowed => Ok(()),
            RightsStatus::Unknown => Err(LedgerError::RightsUnknown(
                self.evidence_id.clone(),
            )),
            RightsStatus::Restricted => Err(LedgerError::RightsRestricted(
                self.evidence_id.clone(),
            )),
        }
    }
}

/// Distinguishes source-event time from when that observation became known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventHeader {
    pub event_id: String,
    /// Stable vessel-call ID or cargo-movement record ID.
    pub entity_id: String,
    pub event_class: EventClass,
    pub event_tick: u64,
    pub observed_tick: u64,
    /// A correction must explicitly supersede one prior event ID.
    pub supersedes_event_id: Option<String>,
    pub source: SourceEvidence,
}

pub trait LedgerObservation {
    fn header(&self) -> &EventHeader;
    fn semantic_kind(&self) -> &'static str;
    fn delta(&self) -> i128;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VesselEventKind {
    EligibleArrival,
    BerthEntry,
    Diversion,
    Cancellation,
    OtherDeclaredExit,
}

impl VesselEventKind {
    const fn delta(self) -> i128 {
        match self {
            Self::EligibleArrival => 1,
            Self::BerthEntry
            | Self::Diversion
            | Self::Cancellation
            | Self::OtherDeclaredExit => -1,
        }
    }

    const fn semantic_kind(self) -> &'static str {
        match self {
            Self::EligibleArrival => "eligible-arrival",
            Self::BerthEntry => "berth-entry",
            Self::Diversion => "diversion",
            Self::Cancellation => "cancellation",
            Self::OtherDeclaredExit => "other-declared-exit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselEventObservation {
    pub header: EventHeader,
    pub terminal_id: String,
    pub kind: VesselEventKind,
}

impl LedgerObservation for VesselEventObservation {
    fn header(&self) -> &EventHeader {
        &self.header
    }

    fn semantic_kind(&self) -> &'static str {
        self.kind.semantic_kind()
    }

    fn delta(&self) -> i128 {
        self.kind.delta()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselQueueLedger {
    pub ledger_id: String,
    pub terminal_id: String,
    pub time_basis_id: String,
    pub window: TickWindow,
    /// Queue stock at the start of the half-open interval [start, end).
    pub opening_queue: u64,
    pub observations: Vec<VesselEventObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselQueueReport {
    pub ledger_id: String,
    pub as_of_tick: u64,
    pub opening_queue: u64,
    pub closing_queue: u64,
    pub arrivals: u128,
    pub exits: u128,
    pub peak_queue: u64,
    /// Integral of queue stock over the window, in vessel-ticks.
    pub queue_area_vessel_ticks: u128,
    pub actual_observations_applied: usize,
    pub non_actual_observations_excluded: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CargoUnit {
    PhysicalContainers,
    Teu,
    Tonnes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoMovementKind {
    VesselDischarge,
    GateIn,
    RailIn,
    OtherDeclaredInflow,
    VesselLoad,
    GateOut,
    RailOut,
    OtherDeclaredOutflow,
}

impl CargoMovementKind {
    const fn sign(self) -> i128 {
        match self {
            Self::VesselDischarge
            | Self::GateIn
            | Self::RailIn
            | Self::OtherDeclaredInflow => 1,
            Self::VesselLoad
            | Self::GateOut
            | Self::RailOut
            | Self::OtherDeclaredOutflow => -1,
        }
    }

    const fn semantic_kind(self) -> &'static str {
        match self {
            Self::VesselDischarge => "vessel-discharge",
            Self::GateIn => "gate-in",
            Self::RailIn => "rail-in",
            Self::OtherDeclaredInflow => "other-declared-inflow",
            Self::VesselLoad => "vessel-load",
            Self::GateOut => "gate-out",
            Self::RailOut => "rail-out",
            Self::OtherDeclaredOutflow => "other-declared-outflow",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoMovementObservation {
    pub header: EventHeader,
    pub terminal_id: String,
    /// Stable movement-record ID; corrections keep this ID and supersede the
    /// earlier event explicitly.
    pub movement_id: String,
    pub kind: CargoMovementKind,
    pub quantity: u64,
    pub unit: CargoUnit,
}

impl LedgerObservation for CargoMovementObservation {
    fn header(&self) -> &EventHeader {
        &self.header
    }

    fn semantic_kind(&self) -> &'static str {
        self.kind.semantic_kind()
    }

    fn delta(&self) -> i128 {
        self.kind.sign() * i128::from(self.quantity)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoInventoryLedger {
    pub ledger_id: String,
    pub terminal_id: String,
    pub inventory_scope_id: String,
    pub time_basis_id: String,
    pub window: TickWindow,
    pub unit: CargoUnit,
    pub opening_inventory: u64,
    pub observations: Vec<CargoMovementObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoInventoryReport {
    pub ledger_id: String,
    pub as_of_tick: u64,
    pub unit: CargoUnit,
    pub opening_inventory: u64,
    pub closing_inventory: u64,
    pub inbound_quantity: u128,
    pub outbound_quantity: u128,
    pub peak_inventory: u64,
    /// Integral of inventory stock over the window, in unit-ticks.
    pub inventory_area_unit_ticks: u128,
    pub actual_observations_applied: usize,
    pub non_actual_observations_excluded: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    EmptyLedgerId,
    EmptyTerminalId,
    EmptyTimeBasis,
    EmptyInventoryScope,
    InvalidWindow,
    SnapshotBeforeWindow,
    InvalidSourceEvidence(String),
    RightsUnknown(String),
    RightsRestricted(String),
    EmptyEventId,
    EmptyEntityId,
    EmptySupersededEventId,
    DuplicateEventId(String),
    UnknownSupersededEvent(String),
    SupersessionSelfReference(String),
    SupersessionEntityMismatch(String),
    SupersessionKindMismatch(String),
    SupersessionClassMismatch(String),
    SupersessionChronology(String),
    MultipleCorrections(String),
    ConflictingObservation {
        entity_id: String,
        semantic_kind: String,
    },
    ActualEventObservedBeforeItOccurred(String),
    ActualEventOutsideWindow(String),
    TerminalMismatch(String),
    DuplicateVesselExit(String),
    VesselExitBeforeArrival(String),
    NegativeVesselQueue(u64),
    VesselQueueOverflow(u64),
    CargoMovementIdMismatch(String),
    CargoUnitMismatch(String),
    ZeroCargoQuantity(String),
    NegativeCargoInventory(u64),
    CargoInventoryOverflow(u64),
    ArithmeticOverflow,
}

/// Snapshot-visible observations after explicit correction resolution.
///
/// The resolver never chooses between conflicting observations by arrival
/// order. Equivalent semantic events must be linked by supersession; otherwise
/// the snapshot is rejected as ambiguous.
fn resolve_observations<'a, T: LedgerObservation>(
    observations: &'a [T],
    as_of_tick: u64,
) -> Result<Vec<&'a T>, LedgerError> {
    let mut by_id = BTreeMap::<&str, usize>::new();

    for (index, observation) in observations.iter().enumerate() {
        let header = observation.header();
        if header.event_id.trim().is_empty() {
            return Err(LedgerError::EmptyEventId);
        }
        if header.entity_id.trim().is_empty() {
            return Err(LedgerError::EmptyEntityId);
        }
        if by_id
            .insert(header.event_id.as_str(), index)
            .is_some()
        {
            return Err(LedgerError::DuplicateEventId(
                header.event_id.clone(),
            ));
        }
    }

    // Future observations do not influence this information-cutoff snapshot,
    // including their correction graph or later conflicts.
    let visible: Vec<&T> = observations
        .iter()
        .filter(|item| item.header().observed_tick <= as_of_tick)
        .collect();
    let visible_ids: BTreeSet<&str> = visible
        .iter()
        .map(|item| item.header().event_id.as_str())
        .collect();

    let mut child_by_parent = BTreeMap::<&str, &str>::new();
    let mut superseded = BTreeSet::<&str>::new();
    for observation in &visible {
        let header = observation.header();
        let Some(parent_id) = header.supersedes_event_id.as_deref() else {
            continue;
        };
        if parent_id.trim().is_empty() {
            return Err(LedgerError::EmptySupersededEventId);
        }
        if parent_id == header.event_id {
            return Err(LedgerError::SupersessionSelfReference(
                header.event_id.clone(),
            ));
        }
        let Some(parent_index) = by_id.get(parent_id).copied() else {
            return Err(LedgerError::UnknownSupersededEvent(
                parent_id.to_owned(),
            ));
        };
        let parent = &observations[parent_index];
        let parent_header = parent.header();
        if !visible_ids.contains(parent_id) {
            return Err(LedgerError::UnknownSupersededEvent(
                parent_id.to_owned(),
            ));
        }
        if parent_header.entity_id != header.entity_id {
            return Err(LedgerError::SupersessionEntityMismatch(
                header.event_id.clone(),
            ));
        }
        if parent.semantic_kind() != observation.semantic_kind() {
            return Err(LedgerError::SupersessionKindMismatch(
                header.event_id.clone(),
            ));
        }
        if parent_header.event_class != header.event_class {
            return Err(LedgerError::SupersessionClassMismatch(
                header.event_id.clone(),
            ));
        }
        if header.observed_tick < parent_header.observed_tick {
            return Err(LedgerError::SupersessionChronology(
                header.event_id.clone(),
            ));
        }
        if child_by_parent
            .insert(parent_id, header.event_id.as_str())
            .is_some()
        {
            return Err(LedgerError::MultipleCorrections(
                parent_id.to_owned(),
            ));
        }
        superseded.insert(parent_id);
    }

    let active: Vec<&T> = visible
        .into_iter()
        .filter(|item| !superseded.contains(item.header().event_id.as_str()))
        .collect();

    let mut semantic_keys = BTreeSet::new();
    for observation in &active {
        let header = observation.header();
        let key = (
            header.entity_id.as_str(),
            observation.semantic_kind(),
            header.event_class,
        );
        if !semantic_keys.insert(key) {
            return Err(LedgerError::ConflictingObservation {
                entity_id: header.entity_id.clone(),
                semantic_kind: observation.semantic_kind().to_owned(),
            });
        }
        header.source.validate()?;
    }

    Ok(active)
}

fn validate_actual_event(
    header: &EventHeader,
    window: TickWindow,
) -> Result<(), LedgerError> {
    if header.event_class == EventClass::Actual {
        if header.event_tick > header.observed_tick {
            return Err(LedgerError::ActualEventObservedBeforeItOccurred(
                header.event_id.clone(),
            ));
        }
        if header.event_tick < window.start_tick || header.event_tick >= window.end_tick {
            return Err(LedgerError::ActualEventOutsideWindow(
                header.event_id.clone(),
            ));
        }
    }
    Ok(())
}

fn add_area(
    area: &mut u128,
    stock: u64,
    duration_ticks: u64,
) -> Result<(), LedgerError> {
    let segment = u128::from(stock)
        .checked_mul(u128::from(duration_ticks))
        .ok_or(LedgerError::ArithmeticOverflow)?;
    *area = area
        .checked_add(segment)
        .ok_or(LedgerError::ArithmeticOverflow)?;
    Ok(())
}

/// Audit a vessel-call queue at a specified information cut-off.
///
/// Queue events are actual-only. Planned/requested/estimated observations are
/// preserved in the input but never change the queue stock. Reporting windows
/// are half-open [start_tick, end_tick); an event at end_tick belongs to the
/// next window. Corrections observed after the cut-off do not rewrite the
/// earlier snapshot.
pub fn audit_vessel_queue(
    ledger: &VesselQueueLedger,
    as_of_tick: u64,
) -> Result<VesselQueueReport, LedgerError> {
    if ledger.ledger_id.trim().is_empty() {
        return Err(LedgerError::EmptyLedgerId);
    }
    if ledger.terminal_id.trim().is_empty() {
        return Err(LedgerError::EmptyTerminalId);
    }
    if ledger.time_basis_id.trim().is_empty() {
        return Err(LedgerError::EmptyTimeBasis);
    }
    if !ledger.window.is_valid() {
        return Err(LedgerError::InvalidWindow);
    }
    if as_of_tick < ledger.window.start_tick {
        return Err(LedgerError::SnapshotBeforeWindow);
    }

    for observation in &ledger.observations {
        if observation.terminal_id != ledger.terminal_id {
            return Err(LedgerError::TerminalMismatch(
                observation.header.event_id.clone(),
            ));
        }
    }

    let active = resolve_observations(&ledger.observations, as_of_tick)?;
    let mut by_tick = BTreeMap::<u64, (u128, u128, usize, usize)>::new();
    let mut exits_by_call = BTreeSet::new();
    let mut arrival_tick_by_call = BTreeMap::new();
    let mut exit_tick_by_call = BTreeMap::new();
    let mut actual_observations_applied = 0_usize;
    let mut non_actual_observations_excluded = 0_usize;

    for observation in active {
        let header = &observation.header;
        if header.event_class != EventClass::Actual {
            non_actual_observations_excluded += 1;
            continue;
        }
        validate_actual_event(header, ledger.window)?;
        let entry = by_tick.entry(header.event_tick).or_default();
        if observation.kind == VesselEventKind::EligibleArrival {
            entry.0 = entry
                .0
                .checked_add(1)
                .ok_or(LedgerError::ArithmeticOverflow)?;
            arrival_tick_by_call.insert(header.entity_id.as_str(), header.event_tick);
        } else {
            if !exits_by_call.insert(header.entity_id.as_str()) {
                return Err(LedgerError::DuplicateVesselExit(
                    header.entity_id.clone(),
                ));
            }
            entry.1 = entry
                .1
                .checked_add(1)
                .ok_or(LedgerError::ArithmeticOverflow)?;
            exit_tick_by_call.insert(header.entity_id.as_str(), header.event_tick);
        }
        entry.2 += 1;
        actual_observations_applied += 1;
    }

    for (call_id, exit_tick) in &exit_tick_by_call {
        if arrival_tick_by_call
            .get(call_id)
            .is_some_and(|arrival_tick| exit_tick < arrival_tick)
        {
            return Err(LedgerError::VesselExitBeforeArrival(
                (*call_id).to_owned(),
            ));
        }
    }

    let mut queue = ledger.opening_queue;
    let mut peak = queue;
    let mut arrivals = 0_u128;
    let mut exits = 0_u128;
    let mut area = 0_u128;
    let mut cursor = ledger.window.start_tick;

    for (tick, (added, removed, _, _)) in by_tick {
        add_area(&mut area, queue, tick - cursor)?;
        let added_u64 = u64::try_from(added).map_err(|_| LedgerError::ArithmeticOverflow)?;
        let removed_u64 = u64::try_from(removed).map_err(|_| LedgerError::ArithmeticOverflow)?;
        queue = queue
            .checked_add(added_u64)
            .ok_or(LedgerError::VesselQueueOverflow(tick))?;
        arrivals = arrivals
            .checked_add(added)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        exits = exits
            .checked_add(removed)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        queue = queue
            .checked_sub(removed_u64)
            .ok_or(LedgerError::NegativeVesselQueue(tick))?;
        peak = peak.max(queue);
        cursor = tick;
    }
    add_area(
        &mut area,
        queue,
        ledger.window.end_tick - cursor,
    )?;

    Ok(VesselQueueReport {
        ledger_id: ledger.ledger_id.clone(),
        as_of_tick,
        opening_queue: ledger.opening_queue,
        closing_queue: queue,
        arrivals,
        exits,
        peak_queue: peak,
        queue_area_vessel_ticks: area,
        actual_observations_applied,
        non_actual_observations_excluded,
    })
}

/// Audit a terminal cargo inventory stream in exactly one unit. Different
/// units must use different ledgers; no implicit box/TEU/tonne conversion is
/// performed.
pub fn audit_cargo_inventory(
    ledger: &CargoInventoryLedger,
    as_of_tick: u64,
) -> Result<CargoInventoryReport, LedgerError> {
    if ledger.ledger_id.trim().is_empty() {
        return Err(LedgerError::EmptyLedgerId);
    }
    if ledger.terminal_id.trim().is_empty() {
        return Err(LedgerError::EmptyTerminalId);
    }
    if ledger.inventory_scope_id.trim().is_empty() {
        return Err(LedgerError::EmptyInventoryScope);
    }
    if ledger.time_basis_id.trim().is_empty() {
        return Err(LedgerError::EmptyTimeBasis);
    }
    if !ledger.window.is_valid() {
        return Err(LedgerError::InvalidWindow);
    }
    if as_of_tick < ledger.window.start_tick {
        return Err(LedgerError::SnapshotBeforeWindow);
    }

    for observation in &ledger.observations {
        if observation.terminal_id != ledger.terminal_id {
            return Err(LedgerError::TerminalMismatch(
                observation.header.event_id.clone(),
            ));
        }
        if observation.movement_id != observation.header.entity_id {
            return Err(LedgerError::CargoMovementIdMismatch(
                observation.header.event_id.clone(),
            ));
        }
        if observation.unit != ledger.unit {
            return Err(LedgerError::CargoUnitMismatch(
                observation.header.event_id.clone(),
            ));
        }
        if observation.quantity == 0 {
            return Err(LedgerError::ZeroCargoQuantity(
                observation.header.event_id.clone(),
            ));
        }
    }

    let active = resolve_observations(&ledger.observations, as_of_tick)?;
    let mut by_tick = BTreeMap::<u64, (u128, u128)>::new();
    let mut actual_observations_applied = 0_usize;
    let mut non_actual_observations_excluded = 0_usize;

    for observation in active {
        let header = &observation.header;
        if header.event_class != EventClass::Actual {
            non_actual_observations_excluded += 1;
            continue;
        }
        validate_actual_event(header, ledger.window)?;
        let amount = u128::from(observation.quantity);
        let entry = by_tick.entry(header.event_tick).or_default();
        if observation.kind.sign() > 0 {
            entry.0 = entry
                .0
                .checked_add(amount)
                .ok_or(LedgerError::ArithmeticOverflow)?;
        } else {
            entry.1 = entry
                .1
                .checked_add(amount)
                .ok_or(LedgerError::ArithmeticOverflow)?;
        }
        actual_observations_applied += 1;
    }

    let mut inventory = ledger.opening_inventory;
    let mut peak = inventory;
    let mut inbound = 0_u128;
    let mut outbound = 0_u128;
    let mut area = 0_u128;
    let mut cursor = ledger.window.start_tick;

    for (tick, (added, removed)) in by_tick {
        add_area(&mut area, inventory, tick - cursor)?;
        let added_u64 = u64::try_from(added).map_err(|_| LedgerError::ArithmeticOverflow)?;
        let removed_u64 = u64::try_from(removed).map_err(|_| LedgerError::ArithmeticOverflow)?;
        inventory = inventory
            .checked_add(added_u64)
            .ok_or(LedgerError::CargoInventoryOverflow(tick))?;
        inbound = inbound
            .checked_add(added)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        outbound = outbound
            .checked_add(removed)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        inventory = inventory
            .checked_sub(removed_u64)
            .ok_or(LedgerError::NegativeCargoInventory(tick))?;
        peak = peak.max(inventory);
        cursor = tick;
    }
    add_area(
        &mut area,
        inventory,
        ledger.window.end_tick - cursor,
    )?;

    Ok(CargoInventoryReport {
        ledger_id: ledger.ledger_id.clone(),
        as_of_tick,
        unit: ledger.unit,
        opening_inventory: ledger.opening_inventory,
        closing_inventory: inventory,
        inbound_quantity: inbound,
        outbound_quantity: outbound,
        peak_inventory: peak,
        inventory_area_unit_ticks: area,
        actual_observations_applied,
        non_actual_observations_excluded,
    })
}

#[must_use]
pub fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
