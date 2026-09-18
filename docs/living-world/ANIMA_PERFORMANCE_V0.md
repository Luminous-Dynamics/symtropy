# ANIMA Performance and Overload Semantics V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Define how ANIMA remains scalable under CPU, memory, persistence, and network pressure without allowing frame timing or engineering load shedding to become hidden behavior policy.

## Core invariant

> Performance pressure may change representation or declared freshness only through explicit qualified semantics; it may not silently rewrite canonical psychology.

Examples of forbidden hidden policy include:

- running cognition only for whichever agents fit the current render frame;
- dropping arbitrary sensor events under load;
- reconstructing a missed historical observation from later hidden world truth;
- reducing memory under allocator pressure and calling it forgetting;
- demoting an individual solely because of camera distance when future-bearing biography exists;
- changing candidate/decision ordering because worker completion order changed.

## Parent authority

Compose with:

- ANIMA canonical scheduling and due-tick semantics;
- bounded agent attention/Umwelt;
- Living World information-sufficient fidelity and ANIMA biography continuity;
- keyed deterministic stochasticity;
- persistence and single-writer authority;
- Observatory trace-level invariance.

This contract does not introduce a camera-distance AI LOD ladder.

## Admissible overload responses

When due work exceeds an engineering budget, an authoritative subsystem may only use a declared response whose semantics are qualified, such as:

1. execute all due work and accept slower wall-clock progress in offline/headless mode;
2. deterministically defer work when the process semantics permit deferral;
3. mark a sample or computation stale/missing when it was not produced;
4. apply a qualified information-sufficient coarse/aggregate closure;
5. pause/fail/diagnose when no semantic-preserving degraded mode exists.

It may not silently substitute current world truth for missed perception or silently skip future-bearing updates.

## Engineering load vs simulated attention

Agent attention is cognitive state and policy. Engineering load shedding is implementation state.

A CPU spike cannot reduce a creature's attention capacity unless a canonical modeled cause changes that capacity. Conversely, attention limits cannot be used as an undocumented excuse to discard engineering work.

## Fidelity is information-based

Distance may help choose candidate representations, but it does not authorize information loss.

A distant named horse can still require persistent rider-specific memory and injury state. A nearby anonymous organism need not run maximal cognition if enabled processes do not consume individual detail.

## Deterministic scheduling under load

Canonical work queues must define stable ordering independent of HashMap iteration, ECS query chunk order, Rayon/OS scheduling, and unrelated task completion.

When deferral is allowed, the rule selecting deferred work is deterministic and versioned. Repeating the same canonical workload/budget policy yields the same due/defer/stale transitions.

## Event-driven activation

Prefer deterministic event-driven wakeups where process semantics permit them, including:

- new qualified percept;
- meaningful regulation threshold transition;
- action completion/interruption;
- scheduled due tick;
- social signal;
- authority or fidelity transition.

A fixed base tick does not imply every agent runs full cognition every tick.

## Population/cohort closure

Batch or cohort simulation is encouraged where Living World fidelity proves sufficiency. Anonymous population state may be compact; future-bearing identity, memory, relationship, injury, or active action may not be absorbed unless the destination representation preserves all surviving semantics.

## Instrumentation

Measure, where relevant:

- wall-clock simulation cost per canonical tick;
- cost per ANIMA/SystemSet stage;
- emissions, propagation, and receptor workload;
- attention candidates considered/retained;
- decisions due/executed/deferred;
- memory entries and bytes per persistent individual;
- social/relationship edges;
- sparse scent/trace chunks;
- action/controller transitions;
- ledger event rate and evidence bytes;
- snapshot/journal growth;
- authority-handoff payload size;
- replicated public ANIMA bandwidth;
- counts by Living World fidelity/identity class;
- deterministic backlog/high-water marks;
- optional FEP/HDC/learned backend costs separately from reference path.

Performance percentiles are diagnostics, not canonical psychology state.

## Benchmark identity

A performance result binds to a sealed semantic workload/scenario manifest plus any hardware/environment fingerprint needed to interpret performance.

Record at least:

- product/profile/policy/scenario identity;
- fixed simulation frequency;
- headless/rendered mode;
- participant/population counts;
- enabled processes;
- trace level;
- workload horizon;
- relevant hardware/toolchain context.

Do not compare numbers from semantically incompatible workloads as if they measured the same system.

## Telemetry invariance

Tracing and profiling are observational.

Minimal, qualification, and detailed debug tracing must produce identical canonical outcomes. Instrumentation must not consume canonical random draws, change deterministic ordering, or feed wall-clock timings into cognition unless running an explicitly noncanonical experiment.

## Presentation isolation

Animation, audio, UI, camera, and debug overlays may reduce quality or frame rate independently. Presentation degradation cannot alter canonical perception, cognition, or action settlement.

## Network pressure

Bandwidth pressure may shed optional telemetry or use qualified replication compression. It may not:

- drop committed biography transitions silently;
- create more than one cognition writer;
- turn predicted client movement into learned experience before canonical settlement;
- expose private belief as social evidence.

## Initial benchmark fixtures

After product gates exist, useful workloads include:

- one full reference horse + rider;
- one canine scent-tracking scenario;
- one deer herd signal-propagation scenario;
- mixed ANIMA Cell;
- increasing persistent-identity census with only process-required subsets active;
- coarse ecological populations with promotion/demotion;
- rendered vs headless equivalence;
- minimal vs detailed tracing equivalence.

Do not freeze arbitrary global agent-count targets before measuring the actual reference workloads.

## Qualification direction

At minimum test:

1. rendered/headless and different presentation FPS yield identical declared canonical ledgers;
2. trace level off/minimal/full leaves behavior unchanged;
3. same overload policy/workload produces the same defer/stale/fidelity decisions;
4. missed sensor work cannot be reconstructed from later hidden truth;
5. load-driven representation transitions preserve required biography;
6. source/container ordering cannot change selection/arbitration;
7. unrelated offscreen computational load cannot perturb another agent except through declared shared-world effects;
8. bounded memory policies remain bounded without allocator-pressure semantic forgetting;
9. long-run evidence/persistence growth is measured and any compaction preserves declared replay semantics;
10. network telemetry/replication pressure does not lose canonical authority events;
11. optional backends cannot weaken reference semantic gates merely to meet performance targets;
12. when no qualified degraded mode exists, overload surfaces explicitly instead of silently changing behavior.

## Non-goals

No premature universal entity-count target, frame-budget-driven hidden cognition skipping, distance-only cognitive fidelity rule, instrumentation-driven behavior, or requirement that every ANIMA process execute every base tick is introduced here.
