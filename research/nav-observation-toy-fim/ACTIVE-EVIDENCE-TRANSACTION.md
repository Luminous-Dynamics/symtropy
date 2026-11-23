# Evidence acquisition transaction composition

**Status:** standalone synthetic composition of three existing software contracts. No physical action is executed and no posterior or authority state is mutated.

**Related owners**
- [#7098 — decision-relevant projection](https://github.com/Luminous-Dynamics/symtropy/pull/7098)
- [#7145 — decision-aware acquisition planner](https://github.com/Luminous-Dynamics/symtropy/pull/7145)
- [#7138 — consume-once acquisition attempt ledger](https://github.com/Luminous-Dynamics/symtropy/pull/7138)
- [#7140 — navigation time-of-validity gate](https://github.com/Luminous-Dynamics/symtropy/pull/7140)
- [#7082 — domain-bound acquisition reliability](https://github.com/Luminous-Dynamics/symtropy/pull/7082)
- [#6900 — dependency-aware fault inventory](https://github.com/Luminous-Dynamics/symtropy/pull/6900)
- [#6470 — integrity/protection-level audit](https://github.com/Luminous-Dynamics/symtropy/issues/6470)
- [#6669 — native mutation boundary](https://github.com/Luminous-Dynamics/symtropy/issues/6669)

## Paradigm: evidence is a staged transaction, not a sensor callback

The same requested sensing action must pass several independent gates before its result can be used to update a navigation belief:

1. **Decision value:** the proposed action is selected by the frozen decision-equivalence partition and resource-constrained Pareto planner.
2. **Attempt identity:** the exact selected action is admitted as a globally consume-once attempt.
3. **Outcome reconciliation:** the executor must return qualified evidence, definitive failure, invalid payload, or unknown. Unknown blocks all new attempts until reconciled.
4. **Time applicability:** a qualified payload must be assessed against physical measurement, receipt, processing, decision and target-valid epochs.
5. **Independent posterior audit:** only a time-applicable nowcast is staged for the separate fault-inventory/posterior/protection-level auditor.
6. **Authority:** this composition never grants navigation authority. Any later mode change remains under the existing independent integrity and supervisory owners; physical movement remains subject to #6669.

This is deliberately not a single scalar "confidence" score. Decision discrimination, acquisition reliability, event identity, temporal applicability and integrity remain separate evidence dimensions.

## Executable composition

The active_evidence_transaction module composes the existing typed modules and preserves their ownership:
- a decision-aware plan must have an action ID, hypothesis inventory commitment, decision-partition commitment and frozen assignments;
- the requested attempt must refer to the exact action selected by that plan;
- a qualified outcome is bound to the same action ID, attempt ID, result ID, payload digest, qualification-receipt digest and source-manifest digest used by the time-validity input;
- the consume-once ledger records the physical attempt outcome and preserves the pre-attempt navigation-state commitment;
- failure and invalid outcomes do not proceed to time evaluation;
- an unknown result produces ReconciliationRequired, while the underlying ledger blocks all other actions until an explicit reconciliation;
- a qualified payload is time-gated before it can be staged for audit.

## Result classes

| Acquisition result | Time claim | Composition result |
|---|---|---|
| Failed/timed out | Any | Acquisition failed; no time report |
| Acquired but invalid | Any | Invalid evidence; no time report |
| Unknown | Any | Reconciliation required; all new attempts blocked |
| Qualified | Missing time assessment | Time assessment missing; unusable |
| Qualified | Stale, unsupported or indeterminate | Temporally inapplicable; unusable |
| Qualified | Event-time estimate | Historical replay only |
| Qualified | Decision-epoch nowcast | Eligible for independent posterior audit |
| Qualified | Future-epoch prediction | Forecast audit only; not an observation |

A successful acquisition is not enough. A fresh receipt timestamp cannot revive a stale measurement. A future-epoch estimate is not reported as an observation at its target epoch.

## Synthetic integration tests

The tests cover:
- an attempt for an action other than the planner's recommendation is rejected before ledger mutation;
- unknown outcome blocks a different action globally until reconciliation;
- valid nowcast reaches posterior-audit staging but cannot update posterior or authority;
- a qualified but stale observation is rejected from current audit and its action remains consumed;
- future predictions and historical estimates take distinct result paths;
- failure, invalidity and missing time assessment never become audit-eligible.

## Boundaries and production follow-up

This is a deterministic in-memory, single-writer composition harness:
- it does not execute a sensor or verify that an acquisition really occurred;
- it does not authenticate digests or receipts;
- it does not make the ledger durable across crash/restart;
- it does not prove cross-process exactly-once physical execution;
- it does not itself recompute or mutate the fault inventory, posterior, protection level or navigation health;
- it does not grant NORMAL or any operating authority;
- it does not call a physical controller or actuator.

Before deployment, the attempt journal needs durable append-before-side-effect semantics, unique attempt enforcement across writers, compare-and-swap/monotonic sequencing and restart reconciliation. Action/plan/inventory/decision-partition commitments must be canonically assembled and independently verified by their respective owners. The time gate must consume Mycelix #435 normalized time and clock-correlation evidence plus #6054 stream ordering. The acquired payload must be independently checked for source lineage, calibration, dependency independence and domain support before the posterior auditor can consume it.

## Acceptance and claim ceiling

A passing suite demonstrates only software composition over the frozen synthetic fixtures and caller-supplied commitments. It does not demonstrate sensor yield, real clock quality, physical truth, rare-event risk, PHMI, mission availability, physical fault containment, airworthiness, safety certification or SOTA superiority.
