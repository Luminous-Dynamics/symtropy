# Planner-neutral restoration action proposal v0.1

**Owner:** Symtropy #6348  
**Case contract:** #6346 / PR #6347  
**External comparators:** #6320  
**Programme:** #5241

## Purpose

All planners must emit the same data-only action proposal against one frozen case digest and one exact observation snapshot. This crate checks proposal shape, actor/action compatibility, stable references, skill/connection constraints and evidence time ordering.

It is not a scheduler or executor. A valid proposal is not necessarily feasible, authorized, safe or executable.

## Action vocabulary

- move_crew: repair crew to a road-network node;
- repair_fault: qualified crew to a known fault;
- move_mobile_source: mobile source to a road node;
- connect_mobile_source / disconnect_mobile_source: mobile source at an explicitly permitted power node;
- set_switch_state: grid-operator actor sets a known switch explicitly open or closed;
- wait: system no-op with no target.

There is deliberately no toggle action. Setting an explicit desired state is idempotent and safe to retry at the proposal layer.

## Evidence and time

Every proposal is bound to:
- the canonical SHA-256 of the full benchmark case;
- an exact observation reference, digest and observed step;
- one decision step equal to the observation step;
- planner ID/revision, source commit, configuration digest and optional prompt digest.

Every action repeats the exact observation reference/decision step and includes one or more precondition evidence references. Evidence may be older than the current step but never from a future step. Each action also carries an authority-requirement reference. A reference names a requirement; this crate does not prove that the actor holds the authority.

## Scope

Validated:
- case digest and canonical case URI;
- planner provenance/config digest shape;
- observation and decision epoch;
- unique action IDs and evidence references;
- known actor IDs and correct actor/action pairings;
- road-node/power-node/fault/switch target references;
- repair crew has the fault's declared skill;
- mobile-source connection target is in its permitted-node set;
- explicit switch target state and no unknown action type.

Not validated:
- radiality after a proposed switch;
- route optimality, time windows, crew/resource capacity or task preemption;
- AC/DC power flow, voltage, protection or energization;
- permit approval or proof of actor authorization;
- execution, service restoration or physical safety.

Those belong to independent authority, transition, electrical and service validators.

## Benchmark fairness

LARA, MILP, CP-SAT, heuristic and Symthaea adapters all emit this same envelope. The independent transition engine receives the same case and observation and decides whether each proposed action can be applied. Invalid proposals stay in the denominator of the invalid-action rate; no adapter may silently repair or discard invalid outputs before scoring.

## Synthetic fixture

The bundled proposal points at the six-node synthetic case from #6346. It proposes one repair action for F23-A by CREW-A at decision step 4, using a step-3 inspection evidence reference. The fixture is only a data-contract example, not a repair recommendation or execution plan.

## Verification

There are 23 authored Rust unit tests. They cover case digest binding, canonical case identity, actor/action pairing, target refs, crew skill, mobile-source permitted nodes, explicit switch state, rejection of toggle, future evidence rejection, exact observation binding, duplicate actions/evidence, authority and provenance digest requirements, and deterministic receipts.

The authoring environment has no Rust toolchain. Tests are authored but not locally executed; CI must pass on the exact PR head before qualification.

## Claim boundary

Proposal shape/reference integrity only. The result status is deliberately `proposal_contract_valid`, never executable or physically feasible.
