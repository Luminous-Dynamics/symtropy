// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed projection of RRVR owner evidence into the pinned TLA+ model.
//!
//! The projected state is supplied by owners. This crate validates its exact
//! shape and selected invariants, then emits a Reflow-compatible state/event
//! JSONL trace plus a separate provenance sidecar. Missing fields are rejected;
//! no owner fact is filled with an optimistic default.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CONTRACT_VERSION: &str = "0.1";
pub const REQUIRED_OWNER_DOMAINS: usize = 5;

pub const STATE_FIELDS: [&str; 43] = [
    "processStatus", "bootEpoch", "controllerEpoch", "stage", "reservationCommitted",
    "oldTicketBootEpoch", "oldTicketControllerEpoch", "oldAccepted", "oldAcceptedBootEpoch",
    "oldAcceptedControllerEpoch", "commandPending", "commandOwnerAttempt",
    "oldPhysicalOutcome", "recoveryPhysicalOutcome", "appliedEffectCount", "persistStatus",
    "storeRecord", "storeCallPending", "effectEvidence", "missionResolution",
    "observerIndependent", "queueFenceBootEpoch", "downstreamFenceBootEpoch", "recoveryPermit",
    "permitUseCount", "ticketStatus", "ticketBootEpoch", "ticketControllerEpoch",
    "ticketAttempt", "m1Current", "m2Current", "ticketBindingValid", "liveBindingValid",
    "recoveryAccepted", "admittedFreshBoot", "admittedFreshController",
    "admittedControllerFence", "admittedDownstreamFence", "admittedM1", "admittedM2",
    "admittedBinding", "naiveReplayCount", "crashCut",
];

pub const MODEL_STATE_FIELD_COUNT: usize = 43;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OwnerEvidenceDomain { ProcessEpoch, Persistence, PhysicalOutcome, ExecutionGraph, Authority }

impl OwnerEvidenceDomain {
    fn as_str(self) -> &'static str {
        match self {
            Self::ProcessEpoch => "process_epoch",
            Self::Persistence => "persistence",
            Self::PhysicalOutcome => "physical_outcome",
            Self::ExecutionGraph => "execution_graph",
            Self::Authority => "authority",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerEvidenceRef {
    pub domain: OwnerEvidenceDomain,
    pub owner_id: String,
    pub owner_revision: String,
    pub evidence_id: String,
    pub raw_artifact_id: String,
    pub raw_artifact_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceClock {
    pub clock_id: String,
    pub boot_epoch_id: String,
    pub unit: String,
    pub value: u64,
    pub resolution: u64,
    pub uncertainty: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSnapshot {
    pub event_id: String,
    pub logical_sequence: u64,
    pub model_action: Option<String>,
    pub model_arguments: Vec<Value>,
    pub predecessor_event_ids: Vec<String>,
    pub source_clock: Option<SourceClock>,
    /// Exact complete TLA variable record after this owner event.
    pub state_s: Value,
    pub owner_evidence: Vec<OwnerEvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionHeader {
    pub bundle_id: String,
    pub system_id: String,
    pub implementation_revision: String,
    pub model_revision: String,
    pub model_source_sha256: String,
    pub model_config_sha256: String,
    pub tla_tool_sha256: String,
    pub fault_profile_revision: String,
    pub max_epoch: u64,
    pub naive_retry_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProjectionRecording {
    pub header: ProjectionHeader,
    pub snapshots: Vec<RuntimeSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    MissingHeaderIdentity, MissingSnapshot, TooManySnapshots, InvalidSequence,
    InvalidEventIdentity, DuplicateEventIdentity, InvalidPredecessor, InvalidModelAction,
    MissingOwnerEvidence, DuplicateOwnerDomain, InvalidOwnerEvidence, InvalidSourceClock,
    MissingOrExtraStateField, InvalidStateFieldType, StateOutOfBounds,
    CommandOwnerMismatch, ReservationStageMismatch, OldAcceptanceLineageMismatch,
    DownstreamFenceInvariant, PersistenceRecordMismatch, DisputedObserverContradiction,
    RecoveryAdmissionInvariant, PermitConsumptionMismatch, RecoveryFromUnqualifiedOutcome,
    IndeterminatePersistenceRecovery, FreshTicketBindingMismatch, InitialStateMismatch,
}

fn nonempty(value: &str) -> bool { !value.trim().is_empty() }
fn valid_sha256(value: &str) -> bool { value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) }
fn str_field<'a>(state: &'a Value, key: &'static str) -> Result<&'a str, ProjectionError> {
    state.get(key).and_then(Value::as_str).filter(|v| nonempty(v)).ok_or(ProjectionError::InvalidStateFieldType)
}
fn bool_field(state: &Value, key: &'static str) -> Result<bool, ProjectionError> {
    state.get(key).and_then(Value::as_bool).ok_or(ProjectionError::InvalidStateFieldType)
}
fn u64_field(state: &Value, key: &'static str) -> Result<u64, ProjectionError> {
    state.get(key).and_then(Value::as_u64).ok_or(ProjectionError::InvalidStateFieldType)
}

pub fn model_init_state() -> Value {
    json!({
        "processStatus":"running","bootEpoch":1,"controllerEpoch":1,"stage":"unreserved",
        "reservationCommitted":false,"oldTicketBootEpoch":1,"oldTicketControllerEpoch":1,
        "oldAccepted":false,"oldAcceptedBootEpoch":0,"oldAcceptedControllerEpoch":0,
        "commandPending":false,"commandOwnerAttempt":"none","oldPhysicalOutcome":"none",
        "recoveryPhysicalOutcome":"none","appliedEffectCount":0,"persistStatus":"definitely_not_committed",
        "storeRecord":false,"storeCallPending":false,"effectEvidence":"none","missionResolution":"none",
        "observerIndependent":true,"queueFenceBootEpoch":0,"downstreamFenceBootEpoch":0,
        "recoveryPermit":"absent","permitUseCount":0,"ticketStatus":"old","ticketBootEpoch":1,
        "ticketControllerEpoch":1,"ticketAttempt":"attempt-1","m1Current":true,"m2Current":true,
        "ticketBindingValid":true,"liveBindingValid":true,"recoveryAccepted":false,
        "admittedFreshBoot":false,"admittedFreshController":false,"admittedControllerFence":false,
        "admittedDownstreamFence":false,"admittedM1":false,"admittedM2":false,"admittedBinding":false,
        "naiveReplayCount":0,"crashCut":"none"
    })
}

fn validate_state(state: &Value, max_epoch: u64) -> Result<(), ProjectionError> {
    let object = state.as_object().ok_or(ProjectionError::MissingOrExtraStateField)?;
    if object.len() != MODEL_STATE_FIELD_COUNT
        || STATE_FIELDS[..MODEL_STATE_FIELD_COUNT].iter().any(|field| !object.contains_key(*field))
    {
        return Err(ProjectionError::MissingOrExtraStateField);
    }
    for key in ["processStatus","stage","commandOwnerAttempt","oldPhysicalOutcome",
        "recoveryPhysicalOutcome","persistStatus","effectEvidence","missionResolution",
        "recoveryPermit","ticketStatus","ticketAttempt","crashCut"] {
        let value = str_field(state, key)?;
        let allowed: &[&str] = match key {
            "processStatus" => &["running","crashed"],
            "stage" => &["unreserved","reserved","dispatch_started","accepted"],
            "commandOwnerAttempt" => &["none","attempt-1","attempt-2"],
            "oldPhysicalOutcome" | "recoveryPhysicalOutcome" => &["none","applied","no_effect","partial"],
            "persistStatus" => &["definitely_not_committed","in_flight","committed","indeterminate_commit"],
            "effectEvidence" | "missionResolution" => &["none","unknown","applied","no_effect","partial","disputed"],
            "recoveryPermit" => &["absent","issued","consumed"],
            "ticketStatus" => &["old","fresh","consumed"],
            "ticketAttempt" => &["attempt-1","attempt-2"],
            "crashCut" => &["none","before_reservation","after_reservation","after_dispatch_start","after_native_accept","after_possible_effect","after_restart_before_fence","after_controller_queue_fence","after_downstream_execution_fence","permit_consumed_before_effect","after_recovery_restart"],
            _ => &[],
        };
        if !allowed.contains(&value) { return Err(ProjectionError::InvalidStateFieldType); }
    }
    for key in ["reservationCommitted","oldAccepted","commandPending","storeRecord","storeCallPending","observerIndependent","m1Current","m2Current","ticketBindingValid","liveBindingValid","recoveryAccepted","admittedFreshBoot","admittedFreshController","admittedControllerFence","admittedDownstreamFence","admittedM1","admittedM2","admittedBinding"] {
        bool_field(state,key)?;
    }
    for key in ["bootEpoch","controllerEpoch","oldTicketBootEpoch","oldTicketControllerEpoch","oldAcceptedBootEpoch","oldAcceptedControllerEpoch","appliedEffectCount","queueFenceBootEpoch","downstreamFenceBootEpoch","permitUseCount","ticketBootEpoch","ticketControllerEpoch","naiveReplayCount"] {
        u64_field(state,key)?;
    }
    let boot = u64_field(state,"bootEpoch")?;
    let controller = u64_field(state,"controllerEpoch")?;
    if max_epoch < 3 || boot == 0 || boot > max_epoch || controller == 0 || controller > max_epoch
        || u64_field(state,"oldTicketBootEpoch")? == 0 || u64_field(state,"oldTicketBootEpoch")? > max_epoch
        || u64_field(state,"oldTicketControllerEpoch")? == 0 || u64_field(state,"oldTicketControllerEpoch")? > max_epoch
        || u64_field(state,"oldAcceptedBootEpoch")? > max_epoch || u64_field(state,"oldAcceptedControllerEpoch")? > max_epoch
        || u64_field(state,"queueFenceBootEpoch")? > max_epoch || u64_field(state,"downstreamFenceBootEpoch")? > max_epoch
        || u64_field(state,"ticketBootEpoch")? == 0 || u64_field(state,"ticketBootEpoch")? > max_epoch
        || u64_field(state,"ticketControllerEpoch")? == 0 || u64_field(state,"ticketControllerEpoch")? > max_epoch
        || u64_field(state,"appliedEffectCount")? > 2 || u64_field(state,"naiveReplayCount")? > 2
        || u64_field(state,"permitUseCount")? > 1
    { return Err(ProjectionError::StateOutOfBounds); }

    let pending = bool_field(state,"commandPending")?;
    let owner = str_field(state,"commandOwnerAttempt")?;
    if pending == (owner == "none") { return Err(ProjectionError::CommandOwnerMismatch); }
    if str_field(state,"stage")? != "unreserved" && !bool_field(state,"reservationCommitted")? {
        return Err(ProjectionError::ReservationStageMismatch);
    }
    if bool_field(state,"oldAccepted")?
        && (!bool_field(state,"reservationCommitted")? || u64_field(state,"oldAcceptedBootEpoch")? != 1
            || u64_field(state,"oldAcceptedControllerEpoch")? != 1)
    { return Err(ProjectionError::OldAcceptanceLineageMismatch); }

    if u64_field(state,"downstreamFenceBootEpoch")? == boot
        && (u64_field(state,"queueFenceBootEpoch")? != boot || pending)
    { return Err(ProjectionError::DownstreamFenceInvariant); }

    let persist = str_field(state,"persistStatus")?;
    if (persist == "committed" && !bool_field(state,"storeRecord")?)
        || (persist == "definitely_not_committed" && bool_field(state,"storeRecord")?)
    { return Err(ProjectionError::PersistenceRecordMismatch); }

    let evidence = str_field(state,"effectEvidence")?;
    if evidence == "disputed" && bool_field(state,"observerIndependent")? {
        return Err(ProjectionError::DisputedObserverContradiction);
    }
    let permit = str_field(state,"recoveryPermit")?;
    let accepted = bool_field(state,"recoveryAccepted")?;
    let use_count = u64_field(state,"permitUseCount")?;
    if (permit == "consumed") != (use_count == 1) || accepted != (permit == "consumed") {
        return Err(ProjectionError::PermitConsumptionMismatch);
    }
    if permit != "absent" && (evidence != "no_effect" || str_field(state,"missionResolution")? != "no_effect"
        || !["committed","definitely_not_committed"].contains(&persist)
        || bool_field(state,"storeCallPending")? || !bool_field(state,"observerIndependent")?
        || !bool_field(state,"m1Current")? || !bool_field(state,"m2Current")?)
    { return Err(ProjectionError::RecoveryFromUnqualifiedOutcome); }
    if persist == "indeterminate_commit" && permit != "absent" {
        return Err(ProjectionError::IndeterminatePersistenceRecovery);
    }
    if str_field(state,"ticketStatus")? == "fresh"
        && (str_field(state,"ticketAttempt")? != "attempt-2"
            || u64_field(state,"ticketBootEpoch")? != boot
            || u64_field(state,"ticketControllerEpoch")? != controller)
    { return Err(ProjectionError::FreshTicketBindingMismatch); }
    if accepted {
        if str_field(state,"oldPhysicalOutcome")? != "no_effect"
            || persist == "indeterminate_commit"
            || !bool_field(state,"ticketBindingValid")?
            || !bool_field(state,"liveBindingValid")?
            || !bool_field(state,"admittedFreshBoot")?
            || !bool_field(state,"admittedFreshController")?
            || !bool_field(state,"admittedControllerFence")?
            || !bool_field(state,"admittedDownstreamFence")?
            || !bool_field(state,"admittedM1")? || !bool_field(state,"admittedM2")?
            || !bool_field(state,"admittedBinding")?
            || u64_field(state,"queueFenceBootEpoch")? != boot
            || u64_field(state,"downstreamFenceBootEpoch")? != boot
        { return Err(ProjectionError::RecoveryAdmissionInvariant); }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    MissingHeaderIdentity, MissingSnapshot, TooManySnapshots, InvalidSequence,
    InvalidEventIdentity, DuplicateEventIdentity, InvalidPredecessor, InvalidModelAction,
    MissingOwnerEvidence, InvalidOwnerEvidence, InvalidSourceClock, DuplicateOwnerDomain,
    MissingOrExtraStateField, InvalidStateFieldType, StateOutOfBounds, CommandOwnerMismatch,
    ReservationStageMismatch, OldAcceptanceLineageMismatch, DownstreamFenceInvariant,
    PersistenceRecordMismatch, DisputedObserverContradiction, PermitConsumptionMismatch,
    RecoveryFromUnqualifiedOutcome, IndeterminatePersistenceRecovery, FreshTicketBindingMismatch,
    RecoveryAdmissionInvariant, InitialStateMismatch,
}

fn nonempty(value: &str) -> bool { !value.trim().is_empty() }
fn valid_sha(value: &str) -> bool { value.len()==64 && value.bytes().all(|b| b.is_ascii_hexdigit()) }
fn known_action(action: &str) -> bool {
    matches!(action,"ReserveAttempt"|"MarkDispatchStarted"|"NativeAcceptOld"|"OldDeviceApplies"
        |"OldDeviceDoesNotApply"|"OldDevicePartiallyApplies"|"BeginEffectRecordCommit"
        |"CommitEffectRecord"|"AbortEffectRecord"|"AcknowledgeStoreCommit"|"AcknowledgeStoreNoCommit"
        |"Crash"|"Restart"|"FenceControllerQueue"|"FenceDownstreamExecution"|"ResolvePersistence"
        |"ObserveOldEffect"|"ObserveUnknownEffect"|"ObserveDisputedEffect"|"ResolveMission"
        |"IssueRecoveryPermit"|"CreateFreshTicket"|"RevokeM1"|"RevokeM2"|"ChangeLiveBinding"
        |"NativeAcceptRecovery"|"RecoveryDeviceApplies"|"RecoveryDeviceDoesNotApply"
        |"RecoveryDevicePartiallyApplies"|"NaiveRetry"|"Next")
}

impl RuntimeProjectionRecording {
    pub fn validate(&self) -> Result<(), ProjectionError> {
        let h=&self.header;
        if [&h.bundle_id,&h.system_id,&h.implementation_revision,&h.model_revision,&h.fault_profile_revision]
            .iter().any(|v| !nonempty(v))
            || !valid_sha(&h.model_source_sha256) || !valid_sha(&h.model_config_sha256)
            || !valid_sha(&h.tla_tool_sha256) || h.max_epoch < 3
        { return Err(ProjectionError::MissingHeaderIdentity); }
        if self.snapshots.is_empty() { return Err(ProjectionError::MissingSnapshot); }
        if self.snapshots.len()>512 { return Err(ProjectionError::TooManySnapshots); }
        let mut ids=BTreeSet::new();
        let required=BTreeSet::from([OwnerEvidenceDomain::ProcessEpoch,OwnerEvidenceDomain::Persistence,
            OwnerEvidenceDomain::PhysicalOutcome,OwnerEvidenceDomain::ExecutionGraph,OwnerEvidenceDomain::Authority]);
        for (i,s) in self.snapshots.iter().enumerate() {
            if !nonempty(&s.event_id) { return Err(ProjectionError::InvalidEventIdentity); }
            if !ids.insert(s.event_id.as_str()) { return Err(ProjectionError::DuplicateEventIdentity); }
            if s.logical_sequence != i as u64+1 { return Err(ProjectionError::InvalidSequence); }
            if i==0 {
                if s.model_action.is_some() || !s.predecessor_event_ids.is_empty()
                    || s.state_s != model_init_state()
                { return Err(ProjectionError::InitialStateMismatch); }
            } else if s.predecessor_event_ids != vec![self.snapshots[i-1].event_id.clone()] {
                return Err(ProjectionError::InvalidPredecessor);
            }
            if s.model_action.as_ref().is_some_and(|action| !known_action(action)) {
                return Err(ProjectionError::InvalidModelAction);
            }
            validate_state(&s.state_s,h.max_epoch)?;
            let domains:BTreeSet<OwnerEvidenceDomain>=s.owner_evidence.iter().map(|e|e.domain).collect();
            if domains != required || s.owner_evidence.len()!=REQUIRED_OWNER_DOMAINS {
                return Err(ProjectionError::MissingOwnerEvidence);
            }
            for e in &s.owner_evidence {
                if !nonempty(&e.owner_id)||!nonempty(&e.owner_revision)||!nonempty(&e.evidence_id)
                    ||!nonempty(&e.raw_artifact_id)||!valid_sha(&e.raw_artifact_sha256) {
                    return Err(ProjectionError::InvalidOwnerEvidence);
                }
            }
            if let Some(clock)=&s.source_clock {
                if !nonempty(&clock.clock_id)||!nonempty(&clock.boot_epoch_id)||!nonempty(&clock.unit)||clock.resolution==0 {
                    return Err(ProjectionError::InvalidSourceClock);
                }
            }
        }
        Ok(())
    }

    pub fn to_tla_trace_ndjson(&self)->Result<String,ProjectionError>{
        self.validate()?;
        let lines:Vec<String>=self.snapshots.iter().map(|s|{
            let mut v=json!({"state":{"s":s.state_s}});
            if let Some(action)=&s.model_action {
                v["event"]=json!(action);
                v["args"]=json!(s.model_arguments);
            }
            serde_json::to_string(&v).expect("serialize JSON value")
        }).collect();
        Ok(format!("{}
",lines.join("
")))
    }

    pub fn to_provenance_sidecar_json(&self)->Result<Value,ProjectionError>{
        self.validate()?;
        let snapshots:Vec<Value>=self.snapshots.iter().map(|s|{
            let state_bytes=serde_json::to_vec(&s.state_s).expect("serialize state");
            json!({
                "event_id":s.event_id,"logical_sequence":s.logical_sequence,
                "model_action":s.model_action,"model_arguments":s.model_arguments,
                "predecessor_event_ids":s.predecessor_event_ids,
                "source_clock":s.source_clock.as_ref().map(|c|json!({
                    "clock_id":c.clock_id,"boot_epoch_id":c.boot_epoch_id,"unit":c.unit,
                    "value":c.value,"resolution":c.resolution,"uncertainty":c.uncertainty
                })),
                "state_sha256":hex(&Sha256::digest(state_bytes)),
                "owner_evidence":s.owner_evidence.iter().map(|e|json!({
                    "domain":e.domain.as_str(),"owner_id":e.owner_id,"owner_revision":e.owner_revision,
                    "evidence_id":e.evidence_id,"raw_artifact_id":e.raw_artifact_id,
                    "raw_artifact_sha256":e.raw_artifact_sha256
                })).collect::<Vec<_>>()
            })
        }).collect();
        Ok(json!({
            "version":CONTRACT_VERSION,"bundle_id":self.header.bundle_id,"system_id":self.header.system_id,
            "implementation_revision":self.header.implementation_revision,"model_revision":self.header.model_revision,
            "model_source_sha256":self.header.model_source_sha256,"model_config_sha256":self.header.model_config_sha256,
            "tla_tool_sha256":self.header.tla_tool_sha256,"fault_profile_revision":self.header.fault_profile_revision,
            "max_epoch":self.header.max_epoch,"naive_retry_enabled":self.header.naive_retry_enabled,
            "snapshots":snapshots
        }))
    }
}

fn hex(bytes:&[u8])->String {
    const D:&[u8;16]=b"0123456789abcdef";
    let mut out=String::with_capacity(bytes.len()*2);
    for b in bytes { out.push(D[(b>>4) as usize] as char); out.push(D[(b&15) as usize] as char); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(c:char)->String { std::iter::repeat(c).take(64).collect() }

    fn owners()->Vec<OwnerEvidenceRef>{
        [OwnerEvidenceDomain::ProcessEpoch,OwnerEvidenceDomain::Persistence,OwnerEvidenceDomain::PhysicalOutcome,
         OwnerEvidenceDomain::ExecutionGraph,OwnerEvidenceDomain::Authority].into_iter().enumerate().map(|(i,d)|OwnerEvidenceRef{
            domain:d,owner_id:format!("owner-{i}"),owner_revision:format!("revision-{i}"),
            evidence_id:format!("evidence-{i}"),raw_artifact_id:format!("artifact-{i}"),
            raw_artifact_sha256:sha(char::from(b'a'+i as u8))
         }).collect()
    }

    fn recording()->RuntimeProjectionRecording{
        RuntimeProjectionRecording{
            header:ProjectionHeader{bundle_id:"runtime-proj-001".into(),system_id:"synthetic-robot".into(),
                implementation_revision:"symtropy-test".into(),model_revision:"RestartSafeEffectContinuity@pinned".into(),
                model_source_sha256:sha('a'),model_config_sha256:sha('b'),tla_tool_sha256:sha('c'),
                fault_profile_revision:"fault-profile-v0.1".into(),max_epoch:4,naive_retry_enabled:false},
            snapshots:vec![RuntimeSnapshot{event_id:"init".into(),logical_sequence:1,model_action:None,
                model_arguments:vec![],predecessor_event_ids:vec![],source_clock:None,
                state_s:model_init_state(),owner_evidence:owners()}]
        }
    }

    #[test] fn emits_exact_model_init_as_reflow_trace_line(){
        assert_eq!(STATE_FIELDS.len(), MODEL_STATE_FIELD_COUNT);
        let r=recording();let line=r.to_tla_trace_ndjson().unwrap();let v:Value=serde_json::from_str(line.trim()).unwrap();
        assert_eq!(v["state"]["s"]["processStatus"],"running");
        assert_eq!(v["state"]["s"]["stage"],"unreserved");assert!(v.get("event").is_none());
    }
    #[test] fn missing_owner_domain_is_not_filled(){
        let mut r=recording();r.snapshots[0].owner_evidence.pop();
        assert_eq!(r.validate(),Err(ProjectionError::MissingOwnerEvidence));
    }
    #[test] fn duplicate_domain_or_invalid_artifact_digest_holds(){
        let mut r=recording();r.snapshots[0].owner_evidence[1].domain=OwnerEvidenceDomain::ProcessEpoch;
        assert_eq!(r.validate(),Err(ProjectionError::MissingOwnerEvidence));
        let mut r=recording();r.snapshots[0].owner_evidence[0].raw_artifact_sha256="bad".into();
        assert_eq!(r.validate(),Err(ProjectionError::InvalidOwnerEvidence));
    }
    #[test] fn indeterminate_persistence_cannot_carry_permit(){
        let mut r=recording();r.snapshots[0].state_s["persistStatus"]=json!("indeterminate_commit");
        r.snapshots[0].state_s["recoveryPermit"]=json!("issued");
        assert_eq!(r.validate(),Err(ProjectionError::RecoveryFromUnqualifiedOutcome));
    }
    #[test] fn applied_unknown_partial_or_disputed_cannot_carry_permit(){
        for outcome in ["applied","unknown","partial","disputed"]{
            let mut r=recording();r.snapshots[0].state_s["effectEvidence"]=json!(outcome);
            r.snapshots[0].state_s["missionResolution"]=json!("no_effect");
            r.snapshots[0].state_s["oldPhysicalOutcome"]=json!("no_effect");
            r.snapshots[0].state_s["recoveryPermit"]=json!("issued");
            assert_eq!(r.validate(),Err(ProjectionError::RecoveryFromUnqualifiedOutcome));
        }
    }
    #[test] fn command_pending_owner_pair_must_match(){
        let mut r=recording();r.snapshots[0].state_s["commandPending"]=json!(true);
        assert_eq!(r.validate(),Err(ProjectionError::CommandOwnerMismatch));
        let mut r=recording();r.snapshots[0].state_s["commandOwnerAttempt"]=json!("attempt-1");
        assert_eq!(r.validate(),Err(ProjectionError::CommandOwnerMismatch));
    }
    #[test] fn downstream_fence_closes_pending_command_and_requires_controller_fence(){
        let mut r=recording();r.snapshots[0].state_s["downstreamFenceBootEpoch"]=json!(1);
        assert_eq!(r.validate(),Err(ProjectionError::DownstreamFenceInvariant));
    }
    #[test] fn recovery_acceptance_requires_exact_no_effect_and_all_current_admissions(){
        let mut r=recording();let s=&mut r.snapshots[0].state_s;
        s["recoveryAccepted"]=json!(true);s["recoveryPermit"]=json!("consumed");s["permitUseCount"]=json!(1);
        s["ticketStatus"]=json!("consumed");s["effectEvidence"]=json!("no_effect");
        s["oldPhysicalOutcome"]=json!("no_effect");s["missionResolution"]=json!("no_effect");
        s["persistStatus"]=json!("committed");s["storeRecord"]=json!(true);
        s["ticketAttempt"]=json!("attempt-2");s["queueFenceBootEpoch"]=json!(1);s["downstreamFenceBootEpoch"]=json!(1);
        s["admittedFreshBoot"]=json!(true);s["admittedFreshController"]=json!(true);
        s["admittedControllerFence"]=json!(true);s["admittedDownstreamFence"]=json!(true);
        s["admittedM1"]=json!(true);s["admittedM2"]=json!(true);s["admittedBinding"]=json!(true);
        assert_eq!(r.validate(),Ok(()));s["m2Current"]=json!(false);
        assert_eq!(r.validate(),Err(ProjectionError::RecoveryAdmissionInvariant));
    }
    #[test] fn initial_state_is_not_an_optimistic_placeholder(){
        let mut r=recording();r.snapshots[0].state_s["observerIndependent"]=json!(false);
        assert_eq!(r.validate(),Err(ProjectionError::InitialStateMismatch));
    }
    #[test] fn later_rows_require_exact_sequence_predecessor_and_known_model_action(){
        let mut r=recording();let mut n=r.snapshots[0].clone();n.event_id="reserve".into();n.logical_sequence=2;
        n.model_action=Some("ReserveAttempt".into());n.predecessor_event_ids=vec!["init".into()];
        n.state_s["stage"]=json!("reserved");n.state_s["reservationCommitted"]=json!(true);r.snapshots.push(n);
        assert_eq!(r.validate(),Ok(()));r.snapshots[1].model_action=Some("Guess".into());
        assert_eq!(r.validate(),Err(ProjectionError::InvalidModelAction));
    }
    #[test] fn incomparable_clock_values_are_retained_without_ordering(){
        let mut r=recording();r.snapshots[0].source_clock=Some(SourceClock{clock_id:"c1".into(),boot_epoch_id:"b1".into(),unit:"ticks".into(),value:100,resolution:1,uncertainty:3});
        assert_eq!(r.validate(),Ok(()));r.snapshots[0].source_clock.as_mut().unwrap().resolution=0;
        assert_eq!(r.validate(),Err(ProjectionError::InvalidSourceClock));
    }
    #[test] fn sidecar_contains_owner_sources_and_state_digest(){
        let r=recording();let sidecar=r.to_provenance_sidecar_json().unwrap();
        assert_eq!(sidecar["snapshots"][0]["owner_evidence"].as_array().unwrap().len(),5);
        assert_eq!(sidecar["snapshots"][0]["state_sha256"].as_str().unwrap().len(),64);
        assert_eq!(sidecar["model_source_sha256"],sha('a'));
    }
}
