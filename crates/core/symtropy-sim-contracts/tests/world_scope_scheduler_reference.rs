// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! WORLD-SCOPE-SCHEDULER-00A executable reference oracle.
//!
//! This file freezes eligibility/frontier algebra only. It owns no simulation
//! clock, domain evolution, causal-event meaning, queue backend, or fidelity.
//!
//! A `DormantUntil(T)` local certificate is negative evidence: it permits the
//! runtime to avoid local polling before T while its bound source remains
//! current. It is not itself a canonical event at T. Independent inbound,
//! forcing, and catch-up obligations may wake the scope earlier.

use std::cmp::Ordering;

use symtropy_sim_contracts::{
    ContractError, DigestAlgorithm, ScopeId, SimInstant, TypedDigest32, WorldInstanceId,
};

const MAX_OBLIGATIONS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WakeKind {
    Local,
    InboundTransit,
    Forcing,
    CatchUp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeState {
    world: WorldInstanceId,
    scope: ScopeId,
    currentness: TypedDigest32,
}

impl ScopeState {
    fn validate(&self) -> Result<(), RefError> {
        self.world.validate()?;
        self.scope.validate()?;
        self.currentness.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LocalForecast {
    /// The domain cannot prove a safe idle interval. The scheduler must ask the
    /// owner again rather than inventing a distant sleep deadline.
    UnknownNextWork,
    /// The owning domain proves that no local canonical work is required before
    /// `not_before`, while `source_currentness` remains current.
    DormantUntil {
        source_currentness: TypedDigest32,
        not_before: SimInstant,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Obligation {
    scope: ScopeId,
    kind: WakeKind,
    at: SimInstant,
    identity: TypedDigest32,
    /// Some obligations depend on the current local continuation state. Others,
    /// such as an independently established inbound transit, remain valid when
    /// unrelated local state advances.
    required_scope_currentness: Option<TypedDigest32>,
}

impl Obligation {
    fn validate_for(&self, state: &ScopeState) -> Result<(), RefError> {
        self.scope.validate()?;
        self.at.validate()?;
        self.identity.validate()?;
        if self.scope != state.scope {
            return Err(RefError::ForeignScopeObligation);
        }
        if let Some(required) = &self.required_scope_currentness {
            required.validate()?;
            if !required.same_typed_value(&state.currentness) {
                return Err(RefError::StaleObligation);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Decision {
    /// Local next-work evidence is unknown or has reached its validity boundary.
    /// `earliest_known_obligation` is advisory evidence for the re-evaluation;
    /// it is not permission to sleep until that instant.
    ReevaluateLocal {
        earliest_known_obligation: Option<SimInstant>,
    },
    /// One or more exact positive obligations are already due. All obligations
    /// at the earliest due instant are returned; the scheduler chooses no winner.
    Due {
        at: SimInstant,
        obligations: Vec<Obligation>,
    },
    /// No canonical work is due now. The runtime may sleep this scope until this
    /// boundary. At the boundary it may need local re-evaluation, positive
    /// obligation execution, or both.
    SleepUntil {
        at: SimInstant,
        reevaluate_local: bool,
        obligations: Vec<Obligation>,
    },
}

fn evaluate(
    state: &ScopeState,
    now: SimInstant,
    local: &LocalForecast,
    obligations: &[Obligation],
) -> Result<Decision, RefError> {
    state.validate()?;
    now.validate()?;
    if obligations.len() > MAX_OBLIGATIONS {
        return Err(RefError::TooManyObligations);
    }

    let mut obligations = obligations.to_vec();
    for obligation in &obligations {
        obligation.validate_for(state)?;
    }
    obligations.sort_by(cmp_obligation);

    let earliest_known_obligation = obligations.iter().map(|value| value.at).min();

    let not_before = match local {
        LocalForecast::UnknownNextWork => {
            return Ok(Decision::ReevaluateLocal {
                earliest_known_obligation,
            });
        }
        LocalForecast::DormantUntil {
            source_currentness,
            not_before,
        } => {
            source_currentness.validate()?;
            not_before.validate()?;
            if !source_currentness.same_typed_value(&state.currentness) {
                return Err(RefError::StaleLocalForecast);
            }
            *not_before
        }
    };

    // A local-domain positive obligation strictly before a certificate claiming
    // no local work before `not_before` is contradictory owner evidence. External
    // obligations are allowed to preempt the local idle certificate.
    if obligations
        .iter()
        .any(|value| value.kind == WakeKind::Local && value.at < not_before)
    {
        return Err(RefError::ContradictoryLocalEvidence);
    }

    if not_before <= now {
        return Ok(Decision::ReevaluateLocal {
            earliest_known_obligation,
        });
    }

    if let Some(earliest_due) = obligations
        .iter()
        .filter(|value| value.at <= now)
        .map(|value| value.at)
        .min()
    {
        return Ok(Decision::Due {
            at: earliest_due,
            obligations: obligations_at(&obligations, earliest_due),
        });
    }

    match earliest_known_obligation {
        Some(positive_at) if positive_at < not_before => Ok(Decision::SleepUntil {
            at: positive_at,
            reevaluate_local: false,
            obligations: obligations_at(&obligations, positive_at),
        }),
        Some(positive_at) if positive_at == not_before => Ok(Decision::SleepUntil {
            at: not_before,
            reevaluate_local: true,
            obligations: obligations_at(&obligations, positive_at),
        }),
        _ => Ok(Decision::SleepUntil {
            at: not_before,
            reevaluate_local: true,
            obligations: vec![],
        }),
    }
}

fn obligations_at(obligations: &[Obligation], at: SimInstant) -> Vec<Obligation> {
    obligations
        .iter()
        .filter(|value| value.at == at)
        .cloned()
        .collect()
}

fn cmp_obligation(left: &Obligation, right: &Obligation) -> Ordering {
    left.at
        .cmp(&right.at)
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| left.scope.cmp(&right.scope))
        .then_with(|| cmp_digest(&left.identity, &right.identity))
        .then_with(|| {
            cmp_optional_digest(
                left.required_scope_currentness.as_ref(),
                right.required_scope_currentness.as_ref(),
            )
        })
}

fn cmp_optional_digest(left: Option<&TypedDigest32>, right: Option<&TypedDigest32>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => cmp_digest(left, right),
    }
}

fn cmp_digest(left: &TypedDigest32, right: &TypedDigest32) -> Ordering {
    left.domain
        .cmp(&right.domain)
        .then_with(|| digest_algorithm_key(&left.algorithm).cmp(&digest_algorithm_key(&right.algorithm)))
        .then_with(|| left.schema_version.cmp(&right.schema_version))
        .then_with(|| left.value.cmp(&right.value))
}

fn digest_algorithm_key(algorithm: &DigestAlgorithm) -> (u8, &str) {
    match algorithm {
        DigestAlgorithm::Sha256 => (0, ""),
        DigestAlgorithm::Other(name) => (1, name.as_str()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    TooManyObligations,
    ForeignScopeObligation,
    StaleLocalForecast,
    StaleObligation,
    ContradictoryLocalEvidence,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

fn world(value: &str) -> WorldInstanceId {
    WorldInstanceId::parse(value).unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

fn digest(domain: &str, value: &[u8]) -> TypedDigest32 {
    TypedDigest32::sha256(domain, 1, value).unwrap()
}

fn instant(seconds: i64) -> SimInstant {
    SimInstant::new(seconds, 0).unwrap()
}

fn state(scope_id: &str, revision: &[u8]) -> ScopeState {
    ScopeState {
        world: world("world:scheduler-fixture"),
        scope: scope(scope_id),
        currentness: digest("scope.continuation.v1", revision),
    }
}

fn dormant(state: &ScopeState, until: i64) -> LocalForecast {
    LocalForecast::DormantUntil {
        source_currentness: state.currentness.clone(),
        not_before: instant(until),
    }
}

fn obligation(
    state: &ScopeState,
    kind: WakeKind,
    at: i64,
    name: &[u8],
    binds_local_currentness: bool,
) -> Obligation {
    Obligation {
        scope: state.scope.clone(),
        kind,
        at: instant(at),
        identity: digest("scope.obligation.v1", name),
        required_scope_currentness: binds_local_currentness.then(|| state.currentness.clone()),
    }
}

#[test]
fn qualified_local_dormancy_avoids_polling_before_boundary() {
    let mars = state("mars", b"r1");
    let result = evaluate(&mars, instant(10), &dormant(&mars, 100), &[]).unwrap();

    assert_eq!(
        result,
        Decision::SleepUntil {
            at: instant(100),
            reevaluate_local: true,
            obligations: vec![],
        }
    );
}

#[test]
fn earlier_inbound_transit_preempts_later_local_dormancy() {
    let mars = state("mars", b"r1");
    let inbound = obligation(
        &mars,
        WakeKind::InboundTransit,
        20,
        b"earth-to-mars-signal",
        false,
    );

    assert_eq!(
        evaluate(&mars, instant(10), &dormant(&mars, 100), &[inbound.clone()]).unwrap(),
        Decision::SleepUntil {
            at: instant(20),
            reevaluate_local: false,
            obligations: vec![inbound],
        }
    );
}

#[test]
fn unknown_local_work_cannot_be_parked_until_known_external_event() {
    let mars = state("mars", b"r1");
    let forcing = obligation(&mars, WakeKind::Forcing, 1_000, b"storm", false);

    assert_eq!(
        evaluate(
            &mars,
            instant(10),
            &LocalForecast::UnknownNextWork,
            &[forcing],
        )
        .unwrap(),
        Decision::ReevaluateLocal {
            earliest_known_obligation: Some(instant(1_000)),
        }
    );
}

#[test]
fn expired_dormancy_certificate_requires_owner_reevaluation() {
    let mars = state("mars", b"r1");

    assert_eq!(
        evaluate(&mars, instant(100), &dormant(&mars, 100), &[]).unwrap(),
        Decision::ReevaluateLocal {
            earliest_known_obligation: None,
        }
    );
}

#[test]
fn equal_time_obligations_return_a_set_not_a_scheduler_chosen_winner() {
    let scope_state = state("local:a", b"r1");
    let inbound = obligation(
        &scope_state,
        WakeKind::InboundTransit,
        20,
        b"inbound",
        false,
    );
    let forcing = obligation(&scope_state, WakeKind::Forcing, 20, b"forcing", false);
    let catch_up = obligation(&scope_state, WakeKind::CatchUp, 20, b"catch-up", true);

    let result = evaluate(
        &scope_state,
        instant(20),
        &dormant(&scope_state, 100),
        &[forcing.clone(), catch_up.clone(), inbound.clone()],
    )
    .unwrap();

    let Decision::Due { at, obligations } = result else {
        panic!("expected exact due set");
    };
    assert_eq!(at, instant(20));
    assert_eq!(obligations.len(), 3);
    assert!(obligations.contains(&inbound));
    assert!(obligations.contains(&forcing));
    assert!(obligations.contains(&catch_up));
}

#[test]
fn insertion_order_cannot_change_due_set_or_frontier() {
    let scope_state = state("local:a", b"r1");
    let a = obligation(&scope_state, WakeKind::Forcing, 20, b"a", false);
    let b = obligation(
        &scope_state,
        WakeKind::InboundTransit,
        20,
        b"b",
        false,
    );
    let c = obligation(&scope_state, WakeKind::CatchUp, 50, b"c", true);

    let first = evaluate(
        &scope_state,
        instant(20),
        &dormant(&scope_state, 100),
        &[a.clone(), b.clone(), c.clone()],
    )
    .unwrap();
    let second = evaluate(
        &scope_state,
        instant(20),
        &dormant(&scope_state, 100),
        &[c, b, a],
    )
    .unwrap();

    assert_eq!(first, second);
}

#[test]
fn stale_local_forecast_and_local_bound_obligation_fail_closed() {
    let before = state("local:a", b"r1");
    let after = state("local:a", b"r2");
    let stale_forecast = dormant(&before, 100);
    let stale_local_obligation = obligation(&before, WakeKind::CatchUp, 20, b"catch-up", true);

    assert_eq!(
        evaluate(&after, instant(10), &stale_forecast, &[]),
        Err(RefError::StaleLocalForecast)
    );
    assert_eq!(
        evaluate(
            &after,
            instant(10),
            &dormant(&after, 100),
            &[stale_local_obligation],
        ),
        Err(RefError::StaleObligation)
    );
}

#[test]
fn independent_inbound_obligation_survives_unrelated_local_revision_change() {
    let before = state("local:a", b"r1");
    let after = state("local:a", b"r2");
    let inbound = obligation(
        &before,
        WakeKind::InboundTransit,
        20,
        b"external-transit",
        false,
    );

    assert_eq!(
        evaluate(&after, instant(10), &dormant(&after, 100), &[inbound.clone()]).unwrap(),
        Decision::SleepUntil {
            at: instant(20),
            reevaluate_local: false,
            obligations: vec![inbound],
        }
    );
}

#[test]
fn local_positive_work_cannot_contradict_local_idle_certificate() {
    let scope_state = state("local:a", b"r1");
    let local = obligation(&scope_state, WakeKind::Local, 20, b"local-work", true);

    assert_eq!(
        evaluate(
            &scope_state,
            instant(10),
            &dormant(&scope_state, 100),
            &[local],
        ),
        Err(RefError::ContradictoryLocalEvidence)
    );
}

#[test]
fn external_obligation_at_local_boundary_preserves_both_reasons_to_wake() {
    let scope_state = state("local:a", b"r1");
    let forcing = obligation(&scope_state, WakeKind::Forcing, 100, b"forcing", false);

    assert_eq!(
        evaluate(
            &scope_state,
            instant(10),
            &dormant(&scope_state, 100),
            &[forcing.clone()],
        )
        .unwrap(),
        Decision::SleepUntil {
            at: instant(100),
            reevaluate_local: true,
            obligations: vec![forcing],
        }
    );
}

#[test]
fn high_frequency_local_scope_does_not_wake_dormant_sibling() {
    let hot = state("local:hot", b"hot-r1");
    let cold = state("local:cold", b"cold-r1");
    let hot_tick = obligation(&hot, WakeKind::Local, 11, b"fixed-tick", true);

    let hot_result = evaluate(
        &hot,
        instant(10),
        &LocalForecast::UnknownNextWork,
        &[hot_tick],
    )
    .unwrap();
    let cold_result = evaluate(&cold, instant(10), &dormant(&cold, 1_000), &[]).unwrap();

    assert!(matches!(hot_result, Decision::ReevaluateLocal { .. }));
    assert_eq!(
        cold_result,
        Decision::SleepUntil {
            at: instant(1_000),
            reevaluate_local: true,
            obligations: vec![],
        }
    );
}
