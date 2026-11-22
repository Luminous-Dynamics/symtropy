# Verified Action Composition Protocol v0.1

**Date:** 22 November 2026  
**Tracking:** Symtropy #6698; parent #6669  
**Evidence level:** contract/reference source only. No simulator, HIL, physical or SOTA result is claimed.

## 1. Research objective

The current mission-supervisor reference permits one unresolved external mutation at a time. This deliberately conservative rule prevents later actions from assuming an unknown physical outcome did not occur.

The next opportunity is to increase throughput without weakening that guarantee:

> A bounded set of actions may execute concurrently only when their exact state, actuation, space-time, resource, authority and evidence dependencies are sufficiently qualified to establish non-interference.

The fallback remains serial execution. Unknown footprint information is a conflict, never an empty footprint.

## 2. Why ordinary geometric conflict checking is insufficient

Two robot trajectories can be spatially disjoint while still conflicting through:
- shared objects or task-state predicates;
- a common actuator, tool, controller loop or workholding fixture;
- power bus, battery, thermal, communication or lifting capacity;
- common ODD, mode, platform configuration, clock or mission generation;
- a shared calibration, navigation or observer dependency;
- task precedence or an action that invalidates another action's precondition;
- an aggregate capacity violation involving three or more individually admissible actions.

Therefore the composition validator must combine several independent owner views. A spatially clear result is one required input, not the complete proof.

## 3. Typed action footprint

Each action attempt supplies an immutable footprint:

```text
ActionRef
  action_id
  attempt
  ticket_digest

SharedContext
  full_binding_digest
  authority_epoch
  clock_domain_id
  boot_epoch_id

Footprint
  action_ref
  shared_context
  qualification: Qualified(footprint_digest) | Unknown
  state_accesses[]
  exclusive_claims[]
  capacity_claims[]
```

### State access

A state access names:
- exact object/state key;
- exact object generation/revision;
- Read or Write access.

For a pair of actions A and B, the same object generation conflicts when either side writes:

[
W_A cap (R_B cup W_B) 
e arnothing
quad lor quad
W_B cap (R_A cup W_A) 
e arnothing
]

The same key at different generations is not independent; it is a stale/mismatched state identity and blocks the batch.

### Exclusive resource and time

An exclusive resource claim binds a stable resource ID and a half-open time interval in one qualified time basis. Intersecting intervals on the same exclusive resource conflict. Time values from different clock/time-basis commitments cannot be compared and block concurrency.

### Shared capacity and hyperedges

Capacity claims bind resource ID, unit profile, capacity profile, time basis, demand and total qualified capacity. The checker sweeps interval start/end events and computes peak concurrent demand.

For half-open intervals, end events are processed before start events at an equal tick. This avoids treating adjacent intervals as overlapping.

This is essential because pairwise tests alone do not capture shared capacity: three actions each below the limit can collectively exceed it.

## 4. Owner-supplied pairwise checks

For every action pair and every required owner family in the exact composition profile, require one check bound to:
- the exact left/right ActionRef, including attempt and ticket digest;
- owner family and profile;
- input commitment and result commitment;
- disposition Clear, Conflict or Unknown.

Only Clear permits that owner dimension to pass. Conflict and Unknown both block the proposed concurrent batch. Missing, duplicate or extraneous owner checks also block.

The owner policy is part of a versioned CompositionProfile:
- profile identity;
- nonzero policy commitment;
- exact unique required owner families;
- maximum supported batch size.

A planner cannot choose a weaker set of required owners in its own request. The profile must be registered/approved by the relevant platform and domain owners.

The Rust reference checks structure, not authenticity. Upstream owner verifiers must independently authenticate and evaluate the space-time, navigation, runtime-assurance, dependency and resource claims. This crate does not recompute geometry or duplicate those systems.

## 5. Shared authority and native application

All members of the initial v0.1 batch must have:
- the same complete shared binding commitment;
- the same authority epoch;
- the same clock domain and boot epoch.

Every action retains its own ticket, action attempt and one-shot identity. A QualifiedBatch never bypasses:
- M1 mission/action/attempt validation;
- M2 configuration/ODD/mode/authority/currentness check;
- #6669 native application linearization and fencing;
- platform-local runtime assurance and safety interlocks;
- independent per-action effect observation.

If the shared authority snapshot changes, every affected action must be revalidated. A controller epoch alone is insufficient unless the owner mapping proves every invalidator advances it in the required order.

## 6. Unresolved outcomes and component quarantine

This v0.1 checker qualifies one fully independent concurrent batch. It does not yet implement conflict-component recovery.

The production profile must eventually maintain an action dependency graph and compute the closure affected by an unknown/partial outcome. Until that closure is independently reconstructible:
- default to mission-wide serial hold;
- do not admit another action based only on a claim that it is unrelated;
- reconcile every action that may have physically applied;
- do not infer that a native controller receipt means a physical effect occurred.

A later version may permit unrelated conflict components to continue, but only after the dependency closure and shared invariants are proven.

## 7. Research comparators

The relevant 2026 dual-robot TAMP paper uses a SAT-based post-plan layer to interleave object-centric action groups under precedence, single-robot exclusivity and handover constraints, then checks candidate overlaps with geometric validation and defers unsafe overlaps. It reports up to 19.8% simulation execution-time reduction and 20–25% on its heterogeneous dual-Franka hardware tasks. This is a useful direct comparator for the *parallelization method*, but not for our additional authority, receipt, persistence and recovery guarantees.

A 2026 multi-agent TAMP survey reviews 60 representative works across discrete task allocation, continuous motion planning, communication and execution uncertainty. A separate 2026 DFGP study reports an 8.5% average lower-bound gap across 18 static allocation scenarios with 5, 10 and 20 robots. These are different benchmark families; do not compare their headline metrics as if interchangeable.

References:
- Dual-robot SAT TAMP parallelization: https://www.sciencedirect.com/science/article/pii/S0921889026004446
- Multi-agent TAMP survey (2026): https://link.springer.com/article/10.1007/s10462-026-11588-5
- Asymptotically optimal multi-robot TAMP (2026): https://arxiv.org/abs/2609.18813
- DFGP multi-robot allocation (2026): https://academic.oup.com/jcde/article/13/8/133/8741942
- Conflict serializability background: https://pld.cs.luc.edu/database/transactions.html

## 8. Matched experiment

Keep task set, hardware, controllers, policy, sensor privilege, energy/resource budget and evaluator identical:

A. Serial supervisor baseline (#6669 default).  
B. Naive parallel scheduling with no conflict certificate (diagnostic-only unsafe control in isolated simulation).  
C. Existing geometry/occupancy + resource reservation checks, without state/effect/authority composition.  
D. Verified composition with exact action footprints, owner pair checks, aggregate constraints, per-action #6669 tickets, independent outcome receipts and quarantine.

Report:
- verified makespan and throughput per robot-hour;
- idle time and concurrent utilization;
- positive batch acceptance and unnecessary-serialization rate;
- conflict false negatives and false positives;
- shared capacity peak and disagreement with exact resource owner;
- collision/contact and physical constraint violations;
- stale/unauthorized application and ticket replay;
- false completion, observer attribution and unresolved effects;
- energy, compute, memory, network, rework and operator intervention.

Research targets:
- zero unsafe concurrent native applications in the frozen negative-control suite;
- 100% agreement with exact resource/occupancy owner oracles;
- no batch admission if a required footprint or owner check is Unknown;
- zero new mutation in an unknown/conflicted dependency component before reconciliation;
- ≥20% median verified-makespan reduction on the pre-registered independent-action subset;
- stretch ≥25% reduction on a matched dual-robot/parallel-TAMP subset;
- valid positive-control acceptance with reported confidence bounds.

The 25% stretch target deliberately challenges the published 20–25% hardware reductions, but only a reproduced head-to-head on the same task/robot/evaluator can establish superiority. If the platform or task differs, report a non-matched replication, not an SOTA claim.

Do not publish one weighted autonomy score. A makespan gain cannot compensate for a false completion or unsafe concurrent action.

## 9. Evidence and claim boundaries

The current reference is a deterministic software validator for supplied typed footprints and owner-check dispositions. It does not:
- authenticate signatures or digests;
- prove owner receipts are truthful;
- compute geometry, swept volumes, dynamic stability or collision probability;
- authorize actuation or create reservations;
- implement dynamic per-component recovery/quarantine;
- establish durable one-shot semantics across restart;
- establish simulator fidelity, HIL, physical safety, field productivity or regulatory approval.

The immediate follow-up is to connect the checker to actual owner receipts and action tickets, then formalize component quarantine separately. The M1 serial default remains untouched until a separately reviewed opt-in profile has passed its own exact-head qualification and domain RTA review.
