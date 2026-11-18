# Shipyard lift execution reconciliation benchmark V0.1

This benchmark evaluates evidence reconciliation—not crane control, lift planning, rigging design, route optimisation, legal authorisation or physical safety. It consumes the exact preflight, authorisation, telemetry, motion and occupancy evidence supplied by their owners.

## 1. Four distinct evaluation layers

### A. Evidence and authority binding

Required invariants:
- a preflight report must be bound to the exact canonical plan-input digest;
- the preflight disposition must be `ready_for_human_review`; it does not grant permission to lift;
- a separate, source-owned authorisation record must bind the exact plan ID/revision and cover the complete execution interval;
- hook-load and gust limits are separately evidenced, and do not come from telemetry measurements;
- source events, calibration and clock-mapping evidence bind exact source IDs/revisions;
- source coverage binds the exact source stream, revision, interval, clock domain and contiguous sequence range;
- event ledger, coverage bundle, policy and report bytes have caller-computed SHA-256 digests.

Hard target: **zero false-conforming reports** when any required plan, authorisation, policy, source, calibration, clock, coverage, sequence or digest binding is missing/mismatched/revoked/stale.

A signature or a lexical SHA-256-shaped digest does not prove sensor calibration, issuer authority or physical truth. Those are verified by the source-owner/authentication workflow and are not inferred by this crate.

### B. Event continuity, clock mapping and uncertainty

Each source event has a source clock domain and a source-to-yard clock-mapping record with exact source and target domains. Its mapped event time is a closed interval `[lower_tick, upper_tick]`, not a point timestamp.

The verifier must:
- reject event-time intervals outside the half-open execution window;
- reject a mapping whose source/target domains do not match;
- require mapping and calibration evidence to be current over the complete mapped interval;
- require ingestion/receipt time not to precede the possible event time;
- compute ingestion lateness conservatively from the earliest possible event tick;
- establish non-overlapping sequence order, abstaining when time uncertainty intervals overlap;
- compute leading, internal and trailing continuous-sensor sample gaps conservatively using worst-case interval endpoints;
- preserve exact retransmissions as auditable, idempotently deduplicated inputs;
- reject conflicting event IDs, source sequence IDs and evidence metadata.

Targets:
- 100% agreement with an independent integer-interval oracle on the frozen timestamp/sequence corpus;
- zero false-conforming under clock reset, drift, mapping invalidation, overlap, sequence gap or excessive uncertainty;
- 100% deterministic replay for identical canonical source inputs and policy;
- no silent imputation or interpolation of missing telemetry.

### C. Operational evidence reconciliation

The V0.1 channels are hook-load upper bound, gust upper bound, crane configuration, independent motion-envelope verdict, independent occupancy verdict, overload alarm and hoist state.

The evaluator reports:
- `ConformingToDeclaredEnvelope` only when all required source coverage is complete under the frozen policy and no deviation is observed;
- `DeviationObserved` when an admitted source explicitly reports a load, gust, configuration, motion or occupancy deviation;
- `Indeterminate` when source/sequence/time/coverage/authorisation evidence is incomplete or inconsistent.

An observed deviation takes precedence over an unrelated unknown interval so it is not hidden; the unknown intervals remain in the report. This does not mean that a session is safe or unsafe in any legal/operational sense.

Benchmark metrics:
- false-conforming count (hard target 0);
- explicit deviation detection rate by type;
- indeterminate rate by cause, never minimised by guessing;
- channel coverage fraction and maximum unobserved interval;
- sequence missing/duplicate/conflicting counts;
- event-time uncertainty width and mapping invalidation detection;
- event-time versus ingestion-time latency p50/p95/p99;
- measured load/gust upper-bound error only against rights-cleared independent reference instrumentation;
- replay determinism, content-addressed input/output receipt stability;
- operator/reviewer time-to-triage and time-to-close, measured separately from machine latency.

Do not combine hard deviations and unknowns into one averaged score. Publish each stratum.

### D. Downstream planning and commercial workflow comparison

This V0.1 is not a hoist-sequence or crane-path optimiser. The 2026 A*HNSA paper by Qi et al. jointly optimises ship-block hoisting sequence and gantry-crane path planning, reporting reductions of 3.51% / 5.33% / 6.79% in transport distance and 3.82% / 5.35% / 7.07% in scheduling time across its small/medium/large cases. Those are literature anchors for a future matched planning benchmark, not a direct comparison to this reconciler:
https://xuebao.sjtu.edu.cn/EN/abstract/abstract51242.shtml

Commercial systems already provide substantial crane planning and telemetry capabilities:
- 3D Lift Plan describes 3D lift visualisation, crane selection, changing-radius capacity monitoring, ground-bearing calculations and rigging/tension calculations: https://www.3dliftplan.com/info/Features.aspx
- ABRA Crane Tracking describes machine telemetry such as running hours, cycles, loads, overloads, brakes and faults, while distinguishing those signals from physical crane/load position: https://www.abracrane.com/products/crane-tracking

Do not claim that this crate replaces either product. The differentiation being tested is vendor-neutral reconciliation that binds exact plan/preflight/authorisation evidence, source telemetry, calibration, clock mapping, coverage and independent motion/occupancy receipts into one replayable result.

A fair future operational comparison must use identical source event histories, sensor/calibration references, clock maps, plan revisions, coverage assertions, independent reference sensors, event windows, lateness policies and failure definitions. Report raw sessions and incomplete/aborted outcomes; do not compare different fleets or sensor configurations without stratification.

## 2. Frozen adversarial strata

The synthetic conformance suite includes:
- plan/preflight digest mismatch and rejected preflight;
- authorisation missing, expired, revoked, wrong-plan and wrong-revision;
- distinct hook-load and gust limit provenance;
- coverage missing, incomplete, duplicate, wrong clock domain or wrong source;
- event identity conflict, exact retransmission and conflicting sequence; both conflicting records must be quarantined and must not affect reported peaks or summaries, regardless of arrival order;
- calibration wrong source revision, stale or revoked;
- source clock mapping wrong source/target domain or wrong mapping revision;
- mapped event-time intervals outside execution, beyond receipt time, overlapping sequence order or too uncertain;
- exact and one-unit-over load/gust bounds;
- sample gap exactly at threshold and one unit beyond (leading/internal/trailing);
- out-of-order, missing and late telemetry;
- crane configuration change, overload alarm, independent motion excursion and occupancy conflict;
- aborted/unknown sessions;
- transport permutation and canonical input/output replay;
- a reported deviation coexisting with an unrelated unknown interval.

For each vector, pin expected disposition and required diagnostic codes. Mutation controls should deliberately:
- ignore an event-source mismatch;
- accept a missing sequence;
- default missing overload telemetry to "no alarm";
- assume a stale sensor calibration is current;
- drop clock-mapping uncertainty;
- round a maximum gap or ingestion delay down;
- accept a rejected preflight;
- silently substitute a different authorisation or operating-limit profile;
- hide a motion/occupancy deviation behind another channel's unknown interval.

The suite must detect every mutation.

## 3. Reproducibility and statistics

V0.1 is deterministic. Do not report stochastic confidence intervals for a deterministic correctness check.

For future real-session trials:
- pre-register sensor/channel inclusion, gap policy, lateness policy, source authority, clock mapping and event window;
- hold out sites, crane configurations, source systems and disruption types;
- retain every session, including aborted, partial and indeterminate records;
- report event-time and ingest-time metrics separately;
- report paired comparisons against independently qualified references;
- identify runner hardware/toolchain, policy revision and exact input/output digests;
- publish exclusions and data-rights limitations.

Synthetic tests establish only the declared software contract. Historical replay, shadow operation and sustained site KPI evidence are separate maturity tiers.

## 4. Standards and scope

[ISO 12480-5:2026](https://www.iso.org/standard/90196.html), published in June 2026, is the bridge/gantry crane safe-use part and is intended to be used with [ISO 12480-1:2024](https://www.iso.org/standard/83637.html). Applicable jurisdiction, crane subtype, site procedure and edition are source-owner/responsible-person decisions. No conformity claim is made by this software.

This reconciliation system has no control authority. It must not be wired into safety PLC or actuator paths as a stop/continue decision engine.

## 5. Release gates

- **G0 — contract**: event, coverage, clock-mapping, authorisation and policy schemas reviewed by source owners.
- **G1 — deterministic software**: Rust 1.96 format, locked/offline tests, all-target check and Clippy; exact-head artifacts retained.
- **G2 — oracle**: independent sequence/clock/interval oracle and all frozen adversarial vectors; zero false-conforming.
- **G3 — replay**: canonical event ledger, coverage bundle, policy and result digests stable under permitted transport reorder.
- **G4 — historical replay**: rights-cleared crane histories plus independent load/position/weather references.
- **G5 — shadow mode**: reconciliation only, no control authority, responsible human review and measured unknown/deviation triage.
- **G6 — field programme**: separate site, operator, engineering, cyber-security, regulatory and applicable standards review.

## 6. Claim ceiling

A conforming result means only that admitted event and coverage records satisfy the declared synthetic reconciliation contract. It does not prove sensor authenticity, complete physical observation, crane health, legal authorisation, safe execution, or whether a lift should continue or stop.
