// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! WORLD-SCOPE-SCHEDULER-00A executable reference oracle.
//!
//! This file freezes sparse causal-frontier algebra only. It owns no simulation
//! clock, domain evolution, causal-event meaning, queue backend, or fidelity.
//!
//! A `DormantUntil(T)` local certificate is negative evidence: it permits the
//! runtime to avoid local polling before T while its bound source remains
//! current. It is not itself a canonical event at T. Independent inbound,
//! forcing, intervention, and catch-up obligations may wake the scope earlier.
//! Overdue obligations retain their original semantic `SimInstant`; dispatching
//! them now never rewrites history to make them appear newly due.

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
    Intervention,
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
    /// No valid local sleep certificate exists. Known future obligations are
    /// advisory evidence only; they do not authorize sleeping until that time.
    ReevaluateLocal {
        earliest_known_obligation: Option<SimInstant>,
    },
    /// Positive work is already due. Every currently overdue obligation is
    /// returned in canonical order with its original semantic due instant.
    /// `reevaluate_local` tells the caller whether local owner evidence must also
    /// be refreshed now.
    WakeNow {
        reevaluate_local: bool,
        obligations: Vec<Obligation>,
    },
    /// No canonical work is due now. The runtime may sleep this scope until this
    /// boundary. At the boundary it may need local re-evaluation, exact positive
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

    let (reevaluate_local_now, local_boundary) = match local {
        LocalForecast::UnknownNextWork => (true, None),
        LocalForecast::DormantUntil {
            source_currentness,
            not_before,
        } => {
            source_currentness.validate()?;
            not_before.validate()?;
            if !source_currentness.same_typed_value(&state.currentness) {
                return Err(RefError::StaleLocalForecast);
            }

            // A local-domain positive obligation strictly before a certificate
            // claiming no local work before `not_before` is contradictory owner
            // evidence. External obligations may legitimately preempt the idle
            // certificate.
            if obligations
                .iter()
                .any(|value| value.kind == WakeKind::Local && value.at < *not_before)
            {
                return Err(RefError::ContradictoryLocalEvidence);
            }

            if *not_before <= now {
                (true, None)
            } else {
                (false, Some(*not_before))
            }
        }
    };

    let overdue: Vec<_> = obligations
        .iter()
        .filter(|value| value.at <= now)
        .cloned()
        .collect();
    if !overdue.is_empty() {
        return Ok(Decision::WakeNow {
            reevaluate_local: reevaluate_local_now,
            obligations: overdue,
        });
    }

    if reevaluate_local_now {
        return Ok(Decision::ReevaluateLocal {
            earliest_known_obligation,
        });
    }

    let local_boundary = local_boundary.expect("non-reevaluate branch must have a boundary");
    match earliest_known_obligation {
        Some(positive_at) if positive_at < local_boundary => Ok(Decision::SleepUntil {
            at: positive_at,
            reevaluate_local: false,
            obligations: obligations_at(&obligations, positive_at),
        }),
        Some(positive_at) if positive_at == local_boundary => Ok(Decision::SleepUntil {
            at: local_boundary,
            reevaluate_local: true,
            obligations: obligations_at(&obligations, positive_at),
        }),
        _ => Ok(Decision::SleepUntil {
            at: local_boundary,
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
        .then_with(|| {
            digest_algorithm_key(&left.algorithm).cmp(&digest_algorithm_key(&right.algorithm))
        })
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
fn unknown_local_work_cannot_be_parked_until_known_future_event() {
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
fn due_external_work_is_not_hidden_by_unknown_local_forecast() {
    let mars = state("mars", b"r1");
    let inbound = obligation(
        &mars,
        WakeKind::InboundTransit,
        8,
        b"already-arrived-signal",
        false,
    );

    assert_eq!(
        evaluate(
            &mars,
            instant(10),
            &LocalForecast::UnknownNextWork,
            &[inbound.clone()],
        )
        .unwrap(),
        Decision::WakeNow {
            reevaluate_local: true,
            obligations: vec![inbound],
        }
    );
}

#[test]
fn expired_dormancy_and_due_external_work_are_both_preserved() {
    let mars = state("mars", b"r1");
    let forcing = obligation(&mars, WakeKind::Forcing, 90, b"storm", false);

    assert_eq!(
        evaluate(&mars, instant(100), &dormant(&mars, 100), &[forcing.clone()]).unwrap(),
        Decision::WakeNow {
            reevaluate_local: true,
            obligations: vec![forcing],
        }
    );
}

#[test]
fn overdue_obligations_keep_original_semantic_times_and_all_return() {
    let scope_state = state("local:a", b"r1");
    let first = obligation(
        &scope_state,
        WakeKind::InboundTransit,
        10,
        b"first",
        false,
    );
    let second = obligation(&scope_state, WakeKind::Forcing, 15, b"second", false);
    let third = obligation(
        &scope_state,
        WakeKind::Intervention,
        20,
        b"third",
        false,
    );

    let result = evaluate(
        &scope_state,
        instant(25),
        &dormant(&scope_state, 100),
        &[third.clone(), first.clone(), second.clone()],
    )
    .unwrap();

    assert_eq!(
        result,
        Decision::WakeNow {
            reevaluate_local: false,
            obligations: vec![first, second, third],
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

    let Decision::WakeNow {
        reevaluate_local,
        obligations,
    } = result
    else {
        panic!("expected exact wake set");
    };
    assert!(!reevaluate_local);
    assert_eq!(obligations.len(), 3);
    assert!(obligations.contains(&inbound));
    assert!(obligations.contains(&forcing));
    assert!(obligations.contains(&catch_up));
}

#[test]
fn insertion_order_cannot_change_wake_set_or_frontier() {
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
    let contradictory = obligation(
        &scope_state,
        WakeKind::Local,
        20,
        b"local-work",
        true,
    );

    assert_eq!(
        evaluate(
            &scope_state,
            instant(10),
            &dormant(&scope_state, 100),
            &[contradictory],
        ),
        Err(RefError::ContradictoryLocalEvidence)
    );
}

#[test]
fn positive_work_at_local_boundary_preserves_both_reasons_to_wake() {
    let scope_state = state("local:a", b"r1");
    let intervention = obligation(
        &scope_state,
        WakeKind::Intervention,
        100,
        b"boundary-intervention",
        false,
    );

    assert_eq!(
        evaluate(
            &scope_state,
            instant(10),
            &dormant(&scope_state, 100),
            &[intervention.clone()],
        )
        .unwrap(),
        Decision::SleepUntil {
            at: instant(100),
            reevaluate_local: true,
            obligations: vec![intervention],
        }
    );
}

#[test]
fn foreign_scope_obligation_fails_closed() {
    let local = state("local:a", b"r1");
    let foreign = state("local:b", b"r1");
    let wrong = obligation(
        &foreign,
        WakeKind::InboundTransit,
        20,
        b"wrong-scope",
        false,
    );

    assert_eq!(
        evaluate(&local, instant(10), &dormant(&local, 100), &[wrong]),
        Err(RefError::ForeignScopeObligation)
    );
}
