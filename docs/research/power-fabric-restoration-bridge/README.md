# Power Fabric Restoration Bridge v0.1

**Status:** research contract / draft schema  
**Issue:** [POWER-FABRIC-RESTORATION-BRIDGE-001 #6235](https://github.com/Luminous-Dynamics/symtropy/issues/6235)  
**Programme:** [POWER-FABRIC-BENCH-001 #5241](https://github.com/Luminous-Dynamics/symtropy/issues/5241)  
**Snapshot:** 2026-11-19

## 1. Purpose

This bridge connects four existing owners without recreating their functionality:

- GRID-ASSET-RECOVERY-001 (#6119) and resumable execution (#6213) own repair tasks, resources, calendars, commitments and work progress.
- CIVDYN-CASCADE-001 (#3894), with draft kernel PR #3896, owns temporal propagation over service dependencies and beneficiary-level service consequences.
- POWER-SYSTEM-RESILIENCE-001 (#3418) and its specialist owners provide physical electrical admissibility, dynamic behaviour, protection and restoration modelling.
- POWER-FABRIC-FEDERATION-001 (#6214) owns actor/scope identity, authority, consent, provenance and cross-scope reconciliation.

The bridge owns only data translation, temporal alignment, evidence classification, mapping receipts and forecast-versus-realized comparison.

## 2. The three timelines are not interchangeable

| Stream | Example | What it proves |
|---|---|---|
| Planned schedule | Repair task estimated to finish at 10:00 | A schedule predicts a work milestone |
| Observed work | Crew reports repair complete at 14:00 with evidence | A work event was observed or attested |
| Modelled/measured service | Independent power model or meter reports delivered service from 15:00 | A service trajectory exists at a defined boundary |

A task finishing does not prove commissioning succeeded. Commissioning passing does not, by itself, prove all downstream consumers receive service. A breaker or valve may remain unavailable; a network bottleneck may persist; redundant capacity may already have preserved service during the outage.

Therefore, a recovery schedule must never be converted directly into a service-availability trace.

## 3. Data flow

1. Import the exact recovery report and bind it to repository, commit, tool/configuration identity and source digest.
2. Retain planned schedule segments as forecast-only records.
3. Import observed work separately, with observation time, source, evidence, freshness and reconciliation state.
4. Import service trajectories from a domain model or measurement source. Keep their native units and declared measurement boundary.
5. Where the source supports it, provide an explicit normalized functionality in [0,1], with a declared numerator, denominator, transformation, model and applicability.
6. Map only a valid, independently sourced normalized service-functionality trace to the appropriate CIVDYN cascade node and exact target ticks.
7. Emit a receipt accounting for every source record and target tick, including rejected, unsupported and unknown rows.
8. Keep forecast and realized results separate; compare only matching service boundaries, demand denominators, horizons and scenario assumptions.

The cascade kernel accepts normalized exogenous own-availability traces. That does not make every normalized value suitable: its meaning, source and boundary must be declared by the bridge.

## 4. Native units and normalization

Never add kW, MWh, m³, litres, Mbps, vehicle capacity or patient capacity into one unqualified scalar.

Keep at least two representations:
- Native service delivered / demand in the service's own units.
- Optional dimensionless functionality in [0,1], only when the source has a defensible denominator and an explicitly named transformation.

For electricity, a declared ratio could be delivered firm service at a named bus divided by a declared demand or service requirement at the same boundary and time interval. Nameplate MW is not delivered MW. A transformer repair milestone is not firm capacity. Network constraints, alternate supply and dispatch must be incorporated by the independent electrical model or measurement process.

For water, use water-specific delivered volume, quality, pressure, treatment and storage boundaries. For telecommunications, use an explicitly defined network service metric and quality/SLA boundary. A powered pump is not automatically water service; a powered radio site is not automatically end-to-end connectivity.

## 5. Time contract

Each bridge dataset declares:
- UTC or explicit IANA time zone;
- start and end instants;
- fixed sampling/aggregation interval;
- source timestamp and original offset where available;
- resampling policy and its version;
- treatment of gaps, duplicates, out-of-order points and daylight-saving transitions;
- observation latency and staleness policy.

Use UTC instants internally. Preserve the original IANA zone for display, jurisdictional calendars and audit. Reject ambiguous wall-clock timestamps unless the source provides an offset or a deterministic resolution rule. Never silently interpolate unknown service into availability.

A service point applies to a half-open interval [start, end). Adjacent points may touch; overlapping points for the same service stream are rejected unless an explicit aggregation rule exists.

## 6. Evidence classes

- planned schedule: simulated/forecast evidence only;
- observed work: observed or explicitly unknown, with provenance;
- modelled service: simulated or inferred, bound to exact model/configuration/data;
- measured service: observed, bound to measurement source, calibration/applicability and freshness.

Do not silently promote a planned forecast to an observation, an observation to a calibrated model, or a simulation to field evidence. Unknown and stale are not zero and are not availability.

## 7. Required mapping receipt

For every source stream and target cascade node, record:
- immutable source artifact digest and revision;
- source row count;
- accepted, rejected, unsupported and unknown row counts;
- target node and target tick count;
- accepted target tick count;
- normalization rule and denominator;
- resampling method;
- rejected-row reasons;
- target artifact digest.

Acceptance requires source accounting to reconcile exactly:
source rows = accepted + rejected + unsupported + unknown rows.

A partial mapping is visible as partial. It cannot be labelled complete.

## 8. Adversarial controls

1. Scheduled repair ends at 10:00; no observed work or service source exists. Expected: forecast only; no restored-service result.
2. Work is observed complete at 14:00; commissioning is absent. Expected: no service-restoration assertion.
3. Commissioning passes at 14:30; independent service source shows pickup at 15:00. Expected: service begins at 15:00, not at task completion.
4. A transformer fails but redundancy maintains customer service. Expected: no false beneficiary outage.
5. An asset has 10 MW nameplate capacity but no independent network-deliverability source. Expected: no normalized service trace.
6. A stale measurement is received after a newer value. Expected: quarantine/reconcile, not overwrite.
7. Two observations conflict. Expected: conflict state and explicit reconciliation.
8. Time zone/offset is absent at a DST-ambiguous instant. Expected: reject or quarantine.
9. Water pump electricity is available but no water, fuel, treatment capacity or pressure exists. Expected: water service remains constrained by typed dependencies.
10. Duplicate/reordered recovery events arrive. Expected: idempotent processing, no double counting.
11. Planned and observed reports share event IDs but have different digests. Expected: preserve both immutable streams.
12. An internal asset and downstream beneficiary both have demand weights. Expected: endpoint accounting rules prevent double-counting of the same service loss.

## 9. Benchmark comparisons

Run five matched variants:
- schedule-only proxy;
- schedule plus observed work/commissioning milestones;
- schedule plus independently modelled service trajectory;
- independently measured service trajectory;
- service cascade with and without redundant/degraded dependency paths.

Report:
- source-to-target semantic mapping accuracy;
- exact source-row and target-tick accounting;
- forecast-to-observed pickup error;
- time to 50%, 90% and 95% service, with sustained recovery rule;
- service-deficit area in native units by service and beneficiary stratum;
- p50/p95/CVaR95 over matched declared scenarios;
- stale/unknown exposure duration;
- schedule churn and forecast revisions;
- false-restoration and false-safe rate (target zero in the frozen negative-control suite);
- time from field observation to reconciled service trajectory.

Do not claim SOTA on an adapter or synthetic fixture. A valid comparison must freeze exact recovery, service-cascade and domain-model revisions; scenario hashes; demand denominators; time grids; solver budgets; hardware; evidence class; and independent expected outputs.

## 10. Current external test baseline

- IEC TS 62898-3-5:2026, published 11 June 2026: HIL, commissioning and periodic testing of microgrid monitoring, control and energy-management systems for grid-connected and isolated microgrids.
- IEEE P2030.7 (PAR approved 27 March 2025) and P2030.8 (PAR approved 26 March 2026) are active revision projects; their predecessors are IEEE 2030.7-2017 and 2030.8-2018.
- May 2026 open-access power-system restoration benchmark paper: six detailed scenarios and an EMT/RMS simulation framework; associated Zenodo dataset 10.5281/zenodo.17340051.
- 2026 rolling-horizon distribution restoration paper: modified IEEE 33-node and 123-node feeder cases; use its reported restoration-time improvement as a reproducibility target under its exact assumptions, not a directly transferable result.

These references complement, but do not replace, the CARM-Plus/PSPLIB/MSLIB/MMLIB scheduling comparisons and the existing CivDyn service-cascade benchmark.

## 11. Qualification boundary

This contract and its synthetic fixture do not implement a production adapter, validate a physical network, establish standards conformity, verify a meter, authorize switching or prove safe return to service. No CI or Rust execution claim is made by this documentation/schema artifact. Independent model owners, operators, asset owners, qualified engineers and competent authorities retain their responsibilities.
