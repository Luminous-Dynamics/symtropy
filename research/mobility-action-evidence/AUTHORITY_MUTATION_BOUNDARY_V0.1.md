# Authority-Linearized Mutation Boundary v0.1

**Status:** research contract / proposed integration seam; implementation and conformance execution pending  
**Tracking:** Symtropy [MOBILITY-MUTATION-BOUNDARY-001 #6669](https://github.com/Luminous-Dynamics/symtropy/issues/6669)  
**Benchmark:** CDAAB (Counterfactual, Authority, and Dependence Assurance Benchmark)  
**Evidence ceiling:** design specification only (L0 documentation); no controller, simulator, HIL, physical or operational qualification is claimed.

## 1. Purpose

This document defines a narrow research contract for one unresolved boundary in verified autonomy:

> The exact authority and context that admitted an action must still be current when the native controller accepts the mutation—not merely when a planner or broker enqueues it.

This is the next integration step after:
- M1 mission supervision, which distinguishes dispatch, acknowledgement, independently evidenced completion, reconciliation and recovery;
- M2 action evidence, which binds an action to mission/action attempt, platform configuration, ODD/mode, authority, clock and navigation evidence;
- the M1/M2 receipt adapter, which projects validated M2 physical-outcome evidence into M1;
- controller-boundary and epoch-fencing reference work, which separately guards dispatch/application and queued commands.

This proposal composes those owners. It does **not** introduce a new mission state machine, action-ticket validator, authority service, controller queue, clock service, observer registry, low-level controller or runtime-assurance system.

## 2. The gap

A pre-dispatch validity check followed by queueing is not sufficient to prove that the action remains valid at application time.

The controller-side epoch fence and bounded epoch queue are useful reference mechanisms, but an integer controller epoch is equivalent to the M2 authority snapshot only if every ticket-invalidating change is mapped to that epoch under a reviewed owner contract and becomes visible before a later command can apply.

A check performed in one component and a mutation performed later in another creates a time-of-check/time-of-use interval. The integration must close that interval at the actual native application boundary.

Keep these events distinct:

```text
intent
  != action proposal
  != M1 active action attempt
  != M2 applicability result
  != queue acceptance
  != native controller application acceptance
  != actuator response
  != independently observed physical effect
  != M1 accepted mission outcome
```

A local queue acknowledgement proves only queue admission. A native-controller receipt proves only the exact semantics its issuing controller contract guarantees. Neither alone establishes physical effect.

## 3. Proposed contract: authority-linearized application

The queue is a transport mechanism, never an authority source. The ticket and its full currentness binding travel to the final application boundary.

Illustrative interface (not a prescribed Rust API):

```text
apply_if_current(
    action_ticket,
    expected_authority_binding,
    live_authority_snapshot,
    trusted_monotonic_instant,
    native_request
) -> ApplicationDisposition
```

The implementation must define one linearization point at the native controller's real command-acceptance boundary. Within the same serialized/atomic operation it must:

1. Validate the ticket's exact mission, subgoal, action, attempt and expected-effect identity against the currently active M1 action.
2. Compare the ticket's complete authority binding with the authoritative live snapshot.
3. Verify platform identity and exact configuration commitment.
4. Verify ODD and operating-mode identity and revision.
5. Verify authority identity, epoch, scope and current revocation state.
6. Verify the source clock/boot and destination comparison basis/epoch plus the exact time-correlation commitment required by the ticket.
7. Verify navigation profile, service, frame, source, sequence and freshness requirements.
8. Check the exclusive expiry boundary against the declared trusted monotonic clock and boot epoch.
9. Atomically consume the one-shot ticket/action-attempt identity so concurrent or replayed submissions cannot apply twice.
10. Accept the command into the native controller path without forwarding it to a second unfenced queue.
11. Emit an application receipt bound to the exact ticket and authority snapshot that won the race.

A mutex around an outer wrapper is not sufficient if another process, driver, transport or hardware queue can bypass that wrapper. The proof claim must name the actual acceptance boundary and every permitted path into it.

### 3.1 Authority binding

The immutable binding must cover, directly or through content-addressed references with versioned semantics:

| Family | Required binding |
|---|---|
| Mission | mission ID/generation, active subgoal, action ID, attempt, expected-effect predicate |
| Platform | stable platform identity, physical configuration revision and digest |
| Operation | ODD identity/revision, operating mode identity/revision |
| Authority | issuer/authority identity, epoch, scope, revocation/currentness evidence |
| Time | source clock and boot/partition identity; destination scale/clock and epoch; correlation profile and commitment; normalized instant, unit and hard uncertainty bound |
| Navigation | profile/service, spatial frame, source identity, source sequence, freshness policy and qualified source time |
| Lease | issue/expiry bounds, monotonic basis and boot identity |
| Replay | immutable ticket/request identity and one-shot consumption key |
| Controller | controller generation/epoch and native application sequence |

Do not replace this binding with a bare timestamp, source label, friendly profile name, request ID or integer epoch. A compact commitment is acceptable only if the full canonical binding is independently reconstructible and validated at the application boundary.

### 3.2 Time and navigation ownership

Mycelix Position remains the owner of time-basis and correlation semantics. The action boundary consumes a qualified correlation result; it does not perform clock conversion itself.

A numeric tick without a basis/clock identity is not comparable time. A change in source clock, boot epoch, correlation profile/commitment, destination epoch or uncertainty envelope invalidates the prior action binding unless the owner explicitly proves otherwise.

Receipt/ingestion time must not substitute for source observation time. A fresh receipt of stale data remains stale.

A navigation source sequence or a higher service label does not prove integrity. Source truth, calibration, dependency independence and navigation-health qualification remain upstream claims owned by their responsible systems.

## 4. Linearization and race semantics

There must be one documented total order for invalidation versus application within the authority domain.

- **Invalidation linearizes first:** application rejects; the native sink receives zero calls.
- **Application linearizes first:** the receipt binds the pre-invalidation snapshot and exact application sequence. A later revocation fences subsequent commands; it does not retroactively erase the accepted application or prove that its physical effects were undone.
- **Concurrent race:** the implementation chooses exactly one of those outcomes at the shared linearization point. No third outcome may both mutate and report rejection.
- **Duplicate/concurrent submission:** the one-shot action-attempt key permits at most one native application.
- **Ambiguous native result:** where the command may have reached the native controller, the result is Unknown/ReconciliationRequired. No new mutation is issued until the existing reconciliation owner establishes the state.
- **Definitive no-effect result:** only a typed, authoritative no-effect outcome may enter the existing M1-approved recovery path.
- **Partial effect:** reconciliation is required before replanning or mutation.
- **Restart:** queued commands from an old controller generation are rejected and recorded. Until durable authority state and restart fencing are separately proven, the system requires fresh authorization after restart; it must not reconstruct permission from a stale local cache.

The application receipt must distinguish at least:
- rejected before native application;
- accepted by native controller (not a physical-effect claim);
- ambiguous after possible native application;
- later independently observed effect disposition.

## 5. Invalidation map

A reviewed mapping is required for every ticket field. For each invalidator, document its owner, how the new state becomes visible to the application boundary, whether it advances a generation, and the maximum interval in which an old ticket could otherwise apply.

| Invalidator | Minimum expected behavior |
|---|---|
| M1 mission generation / active action / attempt changes | Prior ticket cannot apply |
| Platform configuration revision changes | Prior ticket cannot apply |
| ODD or mode identity/revision changes | Prior ticket cannot apply |
| Authority scope or epoch changes; revocation | Prior ticket cannot apply |
| Source/destination clock or boot epoch changes | Prior ticket cannot apply |
| Time-correlation profile/commitment changes or expires | Prior ticket cannot apply |
| Navigation profile/service/frame/source changes | Prior ticket cannot apply |
| Navigation sequence/freshness requirement fails | Prior ticket cannot apply |
| Lease expires, including exact boundary | Prior ticket cannot apply |
| Controller restarts or advances generation | Old queued ticket cannot apply |
| Ticket or attempt already consumed | No second native mutation |

A generation counter may be used as an optimization only after demonstrating that all relevant state owners update it in the required order. Distinct counters must not be presumed synchronized.

## 6. Public conformance corpus

All fixtures must assert both the typed result and the actual native-sink invocation count. Tests that only call a helper and inspect its return value do not establish the boundary.

| Case family | Mutation / event schedule | Expected disposition | Native sink calls |
|---|---|---|---:|
| Positive control | Stable ticket, stable live binding, in-window lease | Applied once | 1 |
| Pre-queue revocation | Revoke after M2 preflight and before enqueue | Rejected stale/revoked | 0 |
| Post-queue revocation | Revoke after enqueue and before application | Rejected stale/revoked | 0 |
| Config change | Change platform config while queued | Rejected configuration mismatch | 0 |
| ODD/mode change | Change ODD or mode revision while queued | Rejected context mismatch | 0 |
| Mission attempt change | Advance M1 action/attempt while queued | Rejected stale attempt | 0 |
| Epoch advance | Advance authority/controller generation while queued | Rejected stale epoch | 0 |
| Clock restart | Change source or comparison boot epoch | Rejected unqualified/stale time | 0 |
| Correlation change | Replace/expire correlation commitment | Rejected time binding | 0 |
| Navigation change | Source/profile/frame/sequence/freshness no longer meets ticket | Rejected navigation binding | 0 |
| Expiry boundary | now equals exclusive expiry | Rejected expired | 0 |
| Expiry while queued | Lease expires before callback applies | Rejected expired | 0 |
| Controller restart | Old-generation queued work is replayed | Rejected stale generation | 0 |
| Duplicate ticket | Replay after successful one-shot consumption | Rejected replay | 0 additional |
| Concurrent duplicate | Two simultaneous applications of same attempt | Exactly one applies; other rejected | 1 total |
| ID/content conflict | Same request ID, different ticket digest | Rejected conflict; ledger unchanged | 0 additional |
| Previous attempt receipt | Delayed receipt names a previous action attempt | M1 rejects stale receipt | 0 new calls |
| Queue capacity | Queue full, no eviction of accepted work | Explicit backpressure/rejection | 0 for rejected item |
| Ambiguous native result | Native call may have happened but reply is lost | Unknown/ReconciliationRequired | At most 1 |
| Forwarding bypass | Wrapper forwards into a second unfenced queue | Conformance failure; no PASS | Must prove no bypass |

Positive controls are mandatory. A reject-all implementation must fail the corpus.

Public fixtures may include expected outcomes as a conformance suite. Confirmatory benchmark runs must additionally use evaluator-held schedules and labels unavailable to the candidate. Freeze scenario identities, timing schedule, seed, controller profile, source pins and evaluator version before execution.

## 7. Matched benchmark arms

Keep the same policy, native controller, platform configuration, sensor privileges and resource budget while comparing:

- **A — Unguarded baseline:** no action-ticket currentness enforcement.
- **B — Preflight only:** M2 applicability validation before queueing.
- **C — Controller epoch/lease only:** existing controller epoch and deadline checks at application.
- **D — Full linearized binding:** M2 ticket plus complete live-snapshot comparison, one-shot consumption and receipt binding at the atomic native application boundary.

Report these dimensions separately (no weighted score):

**Hard integrity gates**
- stale/revoked/configuration-mismatched mutation acceptance;
- replay and duplicate native mutation count;
- false mission completion;
- blind retries after ambiguous/partial outcomes;
- positive-control acceptance and unnecessary-hold rate.

**Latency and utility**
- invalidation-to-fence latency;
- p50/p95/p99 admission and application latency;
- queueing overhead;
- verified task completion and recovery-adjusted completion;
- operator interventions, energy, compute and communication cost per verified completion.

**Evidence quality**
- native receipt completeness;
- exact-head reproducibility and deterministic replay;
- per-invalidation-family denominators;
- unknown/partial disposition correctness;
- cluster-aware confidence intervals and exclusion accounting.

The hard research target is zero invalid native-sink applications in the frozen negative-control corpus, with valid positive controls accepted at the preregistered threshold. This finite deterministic target is not a claim of zero real-world hazard probability.

## 8. Evidence gates

| Gate | Evidence | Claim boundary |
|---|---|---|
| G0 | Reviewed contract, owner map, canonical binding definition | Design only |
| G1 | Unit, adversarial schedule, differential oracle and sink-call-count tests | L0/L1 software conformance |
| G2 | Adapter re-pinned to exact current M1/M2/Mycelix heads; exact-head package tests | Cross-stack software integration only |
| G3 | Real IPC/native-controller application integration; bypass and restart tests | Named software/hardware interface only |
| G4 | Simulator with independent evaluator truth, followed by HIL | Named simulator/HIL configuration only |
| G5 | Approved supervised physical trials, then independent safety/operational review | Only the tested conditions and approval scope |

A queued, skipped, cancelled, stale-head or incomplete run is not a pass. The targeted M2 workflow and broad repository CI are separate evidence. A test receipt from older dependency pins does not qualify a newer integration.

## 9. Current integration pins and known validation boundary (2026-11-21)

These are the latest observed public records at work-package creation:

| Component | Reference | Exact observed result | Limitation |
|---|---|---|---|
| M1 mission supervisor | PR #6499, head `1e29153065c35026a1d07db254ed1815efbee850` | 22 tests; dedicated and broad CI passed | Software contract only |
| M2 action evidence + active assurance | PR #6530, head `97f42a80969dd885040aa4a2681dfb3e5ff26046` | 50 library + 4 differential-oracle tests; targeted workflow passed | Broad CI run was cancelled |
| M1/M2 receipt adapter | PR #6568, head `446c3eb29117c12e0254280cb54437f0bc289176` | 73 tests; targeted and broad CI passed | Pins older M1/M2 commits, so not current-head integration |
| Mycelix Position v2 | PR #6412, head `fb92acb62c3b2b9c040de8ebc34fe371f4efd8a7` | 96 tests in each mirrored workspace | Typed time contract, not device-clock truth or physical qualification |
| Symthaea SOMA adapter | PR #13052, head `a1caaa5685aaac875355143e19c9915175c2fdf4` | 6 isolated adapter tests | Declared Mycelix pin predates the latest time-correlation head; full workspace/provider integration remains open |
| Robotics epoch/application reference | PRs #2875 / #2883 | Source-level reference contracts and authored tests | Not wired to the M2 full binding or a production native sink; tests were not yet reported executed |

The next refreshed M1 + M2 + adapter test composition was estimated at 22 + 54 + 12 = **88 tests** when this document was authored. This is a planning estimate, not an observed result. Refresh source pins, inspect test composition and report the actual result after the integration exists.

## 10. Ownership and non-goals

Continue to use:
- Symtropy M1 for active mission/action/attempt state, reconciliation and approved recovery;
- Symtropy M2 for configuration-bound ticket applicability and physical-effect contract;
- Mycelix Position for qualified time and navigation claim semantics;
- Mycelix provenance/failure-domain owners for source lineage and declared common-cause information;
- Symthaea/domain owners for cognitive policy and domain-specific runtime assurance;
- Mycelix AGENT-009 #803 for durable effect attempts and commit/provider-outcome classification;
- Mycelix AGENT-010 #830 for independent offline reconstruction of bounded authority/effect evidence (its report is an audit artifact, never live authority).
- the native controller and hardware safety layer for the actual command boundary and independent local protection.

This work does not:
- create a new low-level controller, safety-rated emergency stop or certified runtime assurance system;
- authenticate hardware, sensor truth or cryptographic commitments merely by comparing their strings;
- prove dependency inventories are complete or observer paths physically independent;
- synchronize clocks or perform time conversion;
- establish actuator acceptance if a downstream queue is still unfenced;
- make a physical-effect, real-time, simulator-fidelity, HIL, safety, regulatory or SOTA-performance claim.

## 11. Immediate implementation sequence

1. Review and approve the authority-binding field map and invalidation ownership table.
2. Add a thin application-boundary adapter that consumes the existing M2 ticket and existing controller epoch/queue contracts; do not fork either validator.
3. Make the application operation atomic with one-shot ticket consumption and the real native sink invocation.
4. Add the public race corpus and independent sink-call-count oracle.
5. Re-pin the M1/M2 adapter to the current source heads and Mycelix Position commit; rerun exact-head targeted and broad CI separately.
6. Only after the above, add simulator scheduling/fault injection. HIL and physical trials remain later evidence gates.

The research contribution is not a different controller. It is a demonstrable invariant: **no mutation can cross the native application boundary using an action commitment whose complete authority binding is no longer current at that boundary.**