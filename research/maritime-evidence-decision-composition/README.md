# Maritime evidence-to-decision composition v0

**Status:** research contract and frozen synthetic case manifest; not operationally qualified.

This increment tests the seam between existing evidence owners. It does not replace them.

## Existing owners consumed

| Owner | Responsibility | This harness does |
|---|---|---|
| #6974 | source-capture completeness and exact event binding | consumes its disposition and binding |
| #7363 | rights-aware immutable source artifacts and bitemporal revision history | consumes artifact/revision/as-of identity |
| #7386 | source-record claim admission and anti-promotion | preserves claim disposition |
| #7357 | distinct physical, authority and participant-service receipts | checks same-episode/policy/revision binding |
| #7360 | cluster-aware schedule-loss aggregation | checks role and cluster binding |
| #7253 | statistical guarantee and assurance-envelope semantics | preserves guarantee type and gate separation |
| #6662 | paired custody handoffs and receiver closure | consumes custody/receiver disposition |
| #6997 | observable-query policy tree and decision-regret accounting | consumes query universe, cost and latency receipts |
| #7305 | DGT source snapshot profile | preserves source class, rights and valid-time semantics |

## Core contract

A decision evidence bundle is a set of typed references, not a flattened event timeline. The composition reducer returns one of:

- `ELIGIBLE_FOR_DECISION_REVIEW`: every required upstream disposition and binding is admissible for the narrowly declared decision scope. This is not execution authority.
- `ABSTAIN`: evidence is insufficient, stale, not identifiable, not evaluable, unknown, or no feasible authorized action exists.
- `REJECT`: a contradiction, identity mismatch, unsupported claim promotion, rights violation, unauthorized action, or cross-role leakage is detected.

The reducer MUST preserve every upstream disposition and reason. It MUST NOT repair, reinterpret, or strengthen a failed upstream result.

## Binding dimensions

The consumer validates exact equality (or a separately specified, explicit mapping) for:

1. subject/container/visit identity;
2. source, source-native record, artifact revision and raw digest;
3. event class, event time, publication time, retrieval time, valid-time interval and as-of cutoff;
4. source rights and intended use;
5. capture source/configuration/window/reference and exact event binding;
6. admitted claim kind and source-claim disposition;
7. episode, policy, evaluator, loss-contract and operating-regime revisions;
8. physical oracle identity/revision/build digest and authority oracle identity/revision/build digest;
9. participant, permission revision, scope, validity and revocation state;
10. custody handoff, parties, location, transport leg, quantity/unit and intended receiver;
11. cluster membership, calibration/evaluation role and frozen candidate-grid order;
12. statistical guarantee kind, target axis, horizon and stopping rule;
13. query-universe digest, query cost, observation latency and decision-use deadline.

## Gate order and non-compensation

The logical composition is conjunctive, not a weighted score:

`source rights + as-of visibility + capture + claim admission + physical feasibility + authority + participant floors + custody/receiver requirements + statistical claim validity`

Each gate is evaluated independently. A downstream success cannot compensate for an upstream failure. In particular:

- statistical risk cannot override physical infeasibility or unknown physical state;
- physical feasibility cannot grant participant authority;
- an authorized action cannot waive participant service floors;
- terminal departure cannot prove receiver acceptance;
- a digest cannot prove source authenticity, independence or physical truth;
- caller-declared oracle identities cannot prove independence;
- a finite stress set is not an IID calibration population;
- a low expected-risk estimate cannot be relabelled high-probability or time-uniform;
- a source correction unavailable at the decision cutoff cannot rewrite the historical replay.

## Independent reference reducer

The test-only reference reducer must be independently implemented. It must not import production parsers, validators, enums, or helper functions. The production adapter and reference reducer should consume the same frozen case manifest but parse it independently.

For every case, emit:
- case ID and input digest;
- expected and actual disposition;
- stable reason code(s);
- upstream dispositions preserved;
- binding mismatches (field path, expected digest/value class, observed digest/value class);
- guarantee kind and target axis, if applicable;
- exact reducer version and fixture revision.

Never put sensitive raw source data in receipts. Synthetic fixture values only.

## Frozen synthetic corpus

See `fixtures/composition-cases.json`. It contains one valid baseline and 30 named negative/abstention cases. The fixture is a contract manifest, not evidence about a real terminal.

Required future extensions:
- generated permutations of record ordering;
- deletion, duplication, substitution and stale-revision mutations;
- common-cause dependency mutations despite distinct source/oracle IDs;
- time-zone and clock uncertainty;
- unequal cluster-size estimand controls;
- property-based cross-binding tests;
- mutation score with surviving mutations explicitly reported.

## Acceptance gates

- 100% exact agreement with an independently implemented reference reducer.
- Zero unsupported plan/status-to-physical or terminal-to-receiver promotions.
- Zero cross-subject, episode, policy, evaluator, loss-contract or regime reuse.
- Zero post-cutoff evidence in historical as-of replay.
- Zero UNKNOWN / DENIED / INCOMPLETE / NOT_IDENTIFIABLE / NOT_EVALUABLE dispositions converted to success.
- Zero unauthorized actions and participant-floor violations admitted.
- Zero dependence clusters crossing calibration/evaluation.
- Zero guarantee-type or horizon relabelling.
- 100% deterministic byte-identical receipt replay.
- Every rejection/abstention has a stable machine-readable reason.
- Every accepted bundle retains source-rights and oracle-independence limitations.

A zero false-safe result on the frozen corpus is a software acceptance target, not proof of real-world false-safe probability zero.

## Claim ceiling

The v0 corpus is synthetic contract evidence. Passing it does not prove:
- the correctness or independence of any physical/authority oracle;
- that source publishers are authentic or complete;
- physical cargo movement, custody, or receiver delivery;
- DCT-to-DGT exchangeability or regime transfer;
- statistical validity outside the exact theorem assumptions;
- real-port service improvement or operational deployment safety.

No dispatch, berth booking, custody transfer, payment, TOS mutation or other operational action is authorized by this harness.
