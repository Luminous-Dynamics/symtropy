// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001C sparse scheduling/dormancy reference fixture.
//!
//! This composes the six-scope reference cell with the corrected
//! WORLD-SCOPE-SCHEDULER-00A frontier semantics. It owns no simulation clock,
//! domain evolution, queue backend, residency authority, or causal-event
//! ordering semantics.
//!
//! The two central rules are:
//!
//! 1. local `DormantUntil(T)` is negative evidence only: no local work before T
//!    while its source continuation remains current;
//! 2. every already-overdue positive external obligation is preserved with its
//!    original semantic `SimInstant`. Dispatching it now never retimestamps it.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use symtropy_sim_contracts::{
    ContractError, DigestAlgorithm, ScopeId, SimInstant, TypedDigest32, WorldInstanceId,
};

const CELL_SCOPE_COUNT: usize = 6;
const MAX_EXTERNAL_OBLIGATIONS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeCurrentness {
    scope: ScopeId,
    continuation: TypedDigest32,
}

impl ScopeCurrentness {
    fn validate(&self) -> Result<(), RefError> {
        self.scope.validate()?;
        self.continuation.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalSleepCertificate {
    scope: ScopeId,
    source_continuation: TypedDigest32,
    not_before: SimInstant,
}

impl LocalSleepCertificate {
    fn validate_for(&self, current: &ScopeCurrentness) -> Result<(), RefError> {
        self.scope.validate()?;
        self.source_continuation.validate()?;
        self.not_before.validate()?;
        if self.scope != current.scope {
            return Err(RefError::ScopeMismatch);
        }
        if !self
            .source_continuation
            .same_typed_value(&current.continuation)
        {
            return Err(RefError::StaleLocalCertificate);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExternalObligation {
    scope: ScopeId,
    at: SimInstant,
    identity: TypedDigest32,
}

impl ExternalObligation {
    fn validate(&self) -> Result<(), RefError> {
        self.scope.validate()?;
        self.at.validate()?;
        self.identity.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScopeDecision {
    /// Work must be considered now. `obligations` contains every positive
    /// obligation with semantic due time <= the evaluation instant, in canonical
    /// order. Their own `at` values are retained unchanged.
    WakeNow {
        reevaluate_local: bool,
        obligations: Vec<ExternalObligation>,
    },
    /// No work is currently due. At this boundary the scope either needs local
    /// owner re-evaluation, exact positive obligation handling, or both.
    SleepUntil {
        at: SimInstant,
        reevaluate_local: bool,
        obligations: Vec<ExternalObligation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeFrontierEntry {
    scope: ScopeId,
    source_currentness: TypedDigest32,
    local_not_before: SimInstant,
    /// Complete known external set for this bounded reference invocation. The
    /// execution decision may contain only the currently due / next-time subset,
    /// but later obligations are not erased from frontier evidence.
    known_external: Vec<ExternalObligation>,
    decision: ScopeDecision,
}

impl ScopeFrontierEntry {
    fn validate(&self) -> Result<(), RefError> {
        self.scope.validate()?;
        self.source_currentness.validate()?;
        self.local_not_before.validate()?;
        for obligation in &self.known_external {
            obligation.validate()?;
            if obligation.scope != self.scope {
                return Err(RefError::ScopeMismatch);
            }
        }
        validate_decision(&self.scope, &self.decision)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CellFrontier {
    world: WorldInstanceId,
    evaluated_at: SimInstant,
    entries: Vec<ScopeFrontierEntry>,
}

impl CellFrontier {
    fn build(
        world: WorldInstanceId,
        now: SimInstant,
        mut current: Vec<ScopeCurrentness>,
        mut local: Vec<LocalSleepCertificate>,
        mut external: Vec<ExternalObligation>,
    ) -> Result<Self, RefError> {
        world.validate()?;
        now.validate()?;
        if current.len() != CELL_SCOPE_COUNT {
            return Err(RefError::WrongScopeCount);
        }
        if local.len() != current.len() {
            return Err(RefError::MissingLocalCertificate);
        }
        if external.len() > MAX_EXTERNAL_OBLIGATIONS {
            return Err(RefError::TooManyExternalObligations);
        }

        current.sort_by(|left, right| left.scope.cmp(&right.scope));
        local.sort_by(|left, right| left.scope.cmp(&right.scope));
        external.sort_by(cmp_external);
        reject_duplicate_current(&current)?;
        reject_duplicate_local(&local)?;

        for value in &current {
            value.validate()?;
        }
        for value in &external {
            value.validate()?;
            if !current.iter().any(|state| state.scope == value.scope) {
                return Err(RefError::UnknownScope);
            }
        }

        let mut entries = Vec::with_capacity(current.len());
        for state in &current {
            let certificate = local
                .iter()
                .find(|value| value.scope == state.scope)
                .ok_or(RefError::MissingLocalCertificate)?;
            certificate.validate_for(state)?;

            let known_external: Vec<_> = external
                .iter()
                .filter(|value| value.scope == state.scope)
                .cloned()
                .collect();
            let overdue: Vec<_> = known_external
                .iter()
                .filter(|value| value.at <= now)
                .cloned()
                .collect();
            let local_due = certificate.not_before <= now;

            let decision = if local_due || !overdue.is_empty() {
                ScopeDecision::WakeNow {
                    reevaluate_local: local_due,
                    obligations: overdue,
                }
            } else {
                let earliest_external = known_external
                    .iter()
                    .filter(|value| value.at > now)
                    .map(|value| value.at)
                    .min();

                match earliest_external {
                    Some(at) if at < certificate.not_before => ScopeDecision::SleepUntil {
                        at,
                        reevaluate_local: false,
                        obligations: obligations_at(&known_external, at),
                    },
                    Some(at) if at == certificate.not_before => ScopeDecision::SleepUntil {
                        at,
                        reevaluate_local: true,
                        obligations: obligations_at(&known_external, at),
                    },
                    _ => ScopeDecision::SleepUntil {
                        at: certificate.not_before,
                        reevaluate_local: true,
                        obligations: vec![],
                    },
                }
            };

            let entry = ScopeFrontierEntry {
                scope: state.scope.clone(),
                source_currentness: state.continuation.clone(),
                local_not_before: certificate.not_before,
                known_external,
                decision,
            };
            entry.validate()?;
            entries.push(entry);
        }

        entries.sort_by(|left, right| left.scope.cmp(&right.scope));
        let frontier = Self {
            world,
            evaluated_at: now,
            entries,
        };
        frontier.validate()?;
        Ok(frontier)
    }

    fn validate(&self) -> Result<(), RefError> {
        self.world.validate()?;
        self.evaluated_at.validate()?;
        if self.entries.len() != CELL_SCOPE_COUNT {
            return Err(RefError::WrongScopeCount);
        }
        for entry in &self.entries {
            entry.validate()?;
        }
        for pair in self.entries.windows(2) {
            if pair[0].scope >= pair[1].scope {
                return Err(RefError::DuplicateScope);
            }
        }
        Ok(())
    }

    fn entry(&self, scope: &ScopeId) -> Result<&ScopeFrontierEntry, RefError> {
        scope.validate()?;
        self.entries
            .iter()
            .find(|entry| &entry.scope == scope)
            .ok_or(RefError::UnknownScope)
    }

    fn waking_scopes(&self) -> Vec<ScopeId> {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.decision, ScopeDecision::WakeNow { .. }))
            .map(|entry| entry.scope.clone())
            .collect()
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001c.frontier.v2\0");
        push_string(&mut bytes, self.world.as_str());
        push_instant(&mut bytes, self.evaluated_at);
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for entry in &self.entries {
            push_string(&mut bytes, entry.scope.as_str());
            push_digest(&mut bytes, &entry.source_currentness);
            push_instant(&mut bytes, entry.local_not_before);
            bytes.extend_from_slice(&(entry.known_external.len() as u32).to_le_bytes());
            for obligation in &entry.known_external {
                push_external(&mut bytes, obligation);
            }
            match &entry.decision {
                ScopeDecision::WakeNow {
                    reevaluate_local,
                    obligations,
                } => {
                    bytes.push(0);
                    bytes.push(u8::from(*reevaluate_local));
                    bytes.extend_from_slice(&(obligations.len() as u32).to_le_bytes());
                    for obligation in obligations {
                        push_external(&mut bytes, obligation);
                    }
                }
                ScopeDecision::SleepUntil {
                    at,
                    reevaluate_local,
                    obligations,
                } => {
                    bytes.push(1);
                    push_instant(&mut bytes, *at);
                    bytes.push(u8::from(*reevaluate_local));
                    bytes.extend_from_slice(&(obligations.len() as u32).to_le_bytes());
                    for obligation in obligations {
                        push_external(&mut bytes, obligation);
                    }
                }
            }
        }
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001c.frontier.identity.v2",
            2,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PresentationResidency {
    resident: BTreeSet<ScopeId>,
}

impl PresentationResidency {
    fn prewarm(&mut self, scope: ScopeId) -> Result<(), RefError> {
        scope.validate()?;
        self.resident.insert(scope);
        Ok(())
    }
}

fn validate_decision(scope: &ScopeId, decision: &ScopeDecision) -> Result<(), RefError> {
    let obligations = match decision {
        ScopeDecision::WakeNow { obligations, .. } => obligations,
        ScopeDecision::SleepUntil {
            at, obligations, ..
        } => {
            at.validate()?;
            obligations
        }
    };
    for obligation in obligations {
        obligation.validate()?;
        if &obligation.scope != scope {
            return Err(RefError::ScopeMismatch);
        }
    }
    Ok(())
}

fn obligations_at(obligations: &[ExternalObligation], at: SimInstant) -> Vec<ExternalObligation> {
    obligations
        .iter()
        .filter(|value| value.at == at)
        .cloned()
        .collect()
}

fn reject_duplicate_current(values: &[ScopeCurrentness]) -> Result<(), RefError> {
    for pair in values.windows(2) {
        if pair[0].scope == pair[1].scope {
            return Err(RefError::DuplicateScope);
        }
    }
    Ok(())
}

fn reject_duplicate_local(values: &[LocalSleepCertificate]) -> Result<(), RefError> {
    for pair in values.windows(2) {
        if pair[0].scope == pair[1].scope {
            return Err(RefError::DuplicateScope);
        }
    }
    Ok(())
}

fn cmp_external(left: &ExternalObligation, right: &ExternalObligation) -> Ordering {
    left.scope
        .cmp(&right.scope)
        .then_with(|| left.at.cmp(&right.at))
        .then_with(|| cmp_digest(&left.identity, &right.identity))
}

fn cmp_digest(left: &TypedDigest32, right: &TypedDigest32) -> Ordering {
    left.domain
        .cmp(&right.domain)
        .then_with(|| algorithm_key(&left.algorithm).cmp(&algorithm_key(&right.algorithm)))
        .then_with(|| left.schema_version.cmp(&right.schema_version))
        .then_with(|| left.value.cmp(&right.value))
}

fn algorithm_key(algorithm: &DigestAlgorithm) -> (u8, &str) {
    match algorithm {
        DigestAlgorithm::Sha256 => (0, ""),
        DigestAlgorithm::Other(name) => (1, name.as_str()),
    }
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_instant(bytes: &mut Vec<u8>, value: SimInstant) {
    bytes.extend_from_slice(&value.seconds_from_genesis.to_le_bytes());
    bytes.extend_from_slice(&value.nanos.to_le_bytes());
}

fn push_digest(bytes: &mut Vec<u8>, digest: &TypedDigest32) {
    push_string(bytes, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => bytes.push(0),
        DigestAlgorithm::Other(name) => {
            bytes.push(1);
            push_string(bytes, name);
        }
    }
    bytes.extend_from_slice(&digest.schema_version.to_le_bytes());
    bytes.extend_from_slice(&digest.value);
}

fn push_external(bytes: &mut Vec<u8>, obligation: &ExternalObligation) {
    push_string(bytes, obligation.scope.as_str());
    push_instant(bytes, obligation.at);
    push_digest(bytes, &obligation.identity);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    WrongScopeCount,
    MissingLocalCertificate,
    TooManyExternalObligations,
    DuplicateScope,
    UnknownScope,
    ScopeMismatch,
    StaleLocalCertificate,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

fn world() -> WorldInstanceId {
    WorldInstanceId::parse("world:scale-cell-001").unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

fn instant(seconds: i64) -> SimInstant {
    SimInstant::new(seconds, 0).unwrap()
}

fn digest(domain: &str, value: &[u8]) -> TypedDigest32 {
    TypedDigest32::sha256(domain, 1, value).unwrap()
}

fn six_scopes() -> [&'static str; CELL_SCOPE_COUNT] {
    [
        "system:fixture",
        "body:a",
        "region:r0",
        "local:l0",
        "structure:s0",
        "micro:m0",
    ]
}

fn currentness() -> Vec<ScopeCurrentness> {
    six_scopes()
        .into_iter()
        .map(|id| ScopeCurrentness {
            scope: scope(id),
            continuation: digest("scale-cell.scope-currentness.v1", id.as_bytes()),
        })
        .collect()
}

fn certificate(scope_id: &str, at: i64) -> LocalSleepCertificate {
    LocalSleepCertificate {
        scope: scope(scope_id),
        source_continuation: digest("scale-cell.scope-currentness.v1", scope_id.as_bytes()),
        not_before: instant(at),
    }
}

fn base_certificates() -> Vec<LocalSleepCertificate> {
    vec![
        certificate("system:fixture", 1_000),
        certificate("body:a", 1_000),
        certificate("region:r0", 1_000),
        certificate("local:l0", 10),
        certificate("structure:s0", 100),
        certificate("micro:m0", 12),
    ]
}

fn external(scope_id: &str, at: i64, name: &[u8]) -> ExternalObligation {
    ExternalObligation {
        scope: scope(scope_id),
        at: instant(at),
        identity: digest("scale-cell.external-obligation.v1", name),
    }
}

#[test]
fn one_hot_local_frontier_does_not_wake_dormant_ancestors_or_siblings() {
    let frontier = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![],
    )
    .unwrap();

    assert_eq!(frontier.waking_scopes(), vec![scope("local:l0")]);
    assert!(matches!(
        frontier.entry(&scope("system:fixture")).unwrap().decision,
        ScopeDecision::SleepUntil { at, .. } if at == instant(1_000)
    ));
    assert!(matches!(
        frontier.entry(&scope("micro:m0")).unwrap().decision,
        ScopeDecision::SleepUntil { at, .. } if at == instant(12)
    ));
}

#[test]
fn earlier_external_obligation_preempts_dormant_region() {
    let obligation = external("region:r0", 20, b"synthetic-stellar-arrival");
    let before = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![obligation.clone()],
    )
    .unwrap();

    assert_eq!(
        before.entry(&scope("region:r0")).unwrap().decision,
        ScopeDecision::SleepUntil {
            at: instant(20),
            reevaluate_local: false,
            obligations: vec![obligation.clone()],
        }
    );

    let at_arrival = CellFrontier::build(
        world(),
        instant(20),
        currentness(),
        base_certificates(),
        vec![obligation.clone()],
    )
    .unwrap();
    assert_eq!(
        at_arrival.entry(&scope("region:r0")).unwrap().decision,
        ScopeDecision::WakeNow {
            reevaluate_local: false,
            obligations: vec![obligation],
        }
    );
}

#[test]
fn all_overdue_external_obligations_survive_with_original_times() {
    let a = external("region:r0", 5, b"a");
    let b = external("region:r0", 8, b"b");
    let c = external("region:r0", 10, b"c");
    let frontier = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![c.clone(), a.clone(), b.clone()],
    )
    .unwrap();

    let entry = frontier.entry(&scope("region:r0")).unwrap();
    assert_eq!(entry.known_external, vec![a.clone(), b.clone(), c.clone()]);
    assert_eq!(
        entry.decision,
        ScopeDecision::WakeNow {
            reevaluate_local: false,
            obligations: vec![a, b, c],
        }
    );
}

#[test]
fn expired_local_boundary_and_overdue_external_work_preserve_both_reasons() {
    let obligation = external("local:l0", 9, b"external-local-wake");
    let frontier = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![obligation.clone()],
    )
    .unwrap();

    assert_eq!(
        frontier.entry(&scope("local:l0")).unwrap().decision,
        ScopeDecision::WakeNow {
            reevaluate_local: true,
            obligations: vec![obligation],
        }
    );
}

#[test]
fn equal_time_external_work_is_a_set_not_a_selected_winner() {
    let a = external("region:r0", 20, b"a");
    let b = external("region:r0", 20, b"b");
    let frontier = CellFrontier::build(
        world(),
        instant(20),
        currentness(),
        base_certificates(),
        vec![b.clone(), a.clone()],
    )
    .unwrap();

    let ScopeDecision::WakeNow {
        reevaluate_local,
        obligations,
    } = &frontier.entry(&scope("region:r0")).unwrap().decision
    else {
        panic!("region must wake at the external arrival time");
    };
    assert!(!reevaluate_local);
    assert_eq!(obligations.len(), 2);
    assert!(obligations.contains(&a));
    assert!(obligations.contains(&b));
}

#[test]
fn input_order_cannot_change_frontier_identity() {
    let mut current_reversed = currentness();
    current_reversed.reverse();
    let mut local_reversed = base_certificates();
    local_reversed.reverse();
    let a = external("region:r0", 20, b"a");
    let b = external("body:a", 30, b"b");

    let first = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![a.clone(), b.clone()],
    )
    .unwrap();
    let second = CellFrontier::build(
        world(),
        instant(10),
        current_reversed,
        local_reversed,
        vec![b, a],
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.digest().unwrap(), second.digest().unwrap());
}

#[test]
fn stale_local_certificate_fails_closed() {
    let mut certificates = base_certificates();
    let region = certificates
        .iter_mut()
        .find(|value| value.scope == scope("region:r0"))
        .unwrap();
    region.source_continuation = digest("scale-cell.scope-currentness.v1", b"old-region");

    assert_eq!(
        CellFrontier::build(world(), instant(10), currentness(), certificates, vec![]),
        Err(RefError::StaleLocalCertificate)
    );
}

#[test]
fn unknown_external_scope_fails_closed() {
    assert_eq!(
        CellFrontier::build(
            world(),
            instant(10),
            currentness(),
            base_certificates(),
            vec![external("unknown:x", 20, b"foreign")],
        ),
        Err(RefError::UnknownScope)
    );
}

#[test]
fn presentation_prewarm_changes_residency_not_scheduler_frontier() {
    let before = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![],
    )
    .unwrap();
    let before_digest = before.digest().unwrap();

    let mut presentation = PresentationResidency::default();
    presentation.prewarm(scope("micro:m0")).unwrap();
    presentation.prewarm(scope("body:a")).unwrap();
    assert_eq!(presentation.resident.len(), 2);

    let after = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![],
    )
    .unwrap();
    assert_eq!(before, after);
    assert_eq!(before_digest, after.digest().unwrap());
}

#[test]
fn later_external_obligations_remain_bound_even_when_not_next_to_execute() {
    let early = external("region:r0", 20, b"early");
    let late = external("region:r0", 900, b"late");
    let frontier = CellFrontier::build(
        world(),
        instant(10),
        currentness(),
        base_certificates(),
        vec![late.clone(), early.clone()],
    )
    .unwrap();

    let entry = frontier.entry(&scope("region:r0")).unwrap();
    assert_eq!(entry.known_external, vec![early.clone(), late]);
    assert_eq!(
        entry.decision,
        ScopeDecision::SleepUntil {
            at: instant(20),
            reevaluate_local: false,
            obligations: vec![early],
        }
    );
}
