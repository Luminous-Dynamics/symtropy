# Benchmark Protocol v0.1 — South African Corridor Execution

**Status:** executable source-transcription and arithmetic reference; synthetic test mutations; no live operational data.

## 1. Research question

Can we provide a small, reproducible baseline that prevents corridor performance reporting from conflating planned execution, on-time departure and on-time arrival—and can later be joined to qualified route-section, port-call and final-delivery observations?

This v0.1 answers only the first question. It does not test an optimizer.

## 2. Source snapshot

Transnet Rail Infrastructure Manager, *2026 Operational Performance Review*, FY2025/26, operational performance table on report pages 8–9:

https://www.transnet.net/getFile.ashx?id=27812

The CSV records the six published corridor rows for:
- train plan vs ran, target and actual;
- on-time departures (OTD) including 80s, target and actual;
- OTD excluding 80s, target and actual;
- on-time arrivals (OTA), target and actual.

The source reports percentage values. It does not provide the underlying counts, per-train event records, denominators, route-level breakdowns or uncertainty intervals in this table.

### Source-label discipline

The source's two OTD labels are retained exactly as `including 80s` and `excluding 80s`. The report narrative discusses the corresponding measures as including or excluding exceptions, but this corpus does not define code 80, its event-level meaning, eligibility rule or denominator. Do not infer those details, and do not collapse the two percentages into one score.

### Period and scope

- Observation period: FY2025/26.
- Geographic scope: aggregate corridor values as published by TRIM.
- Unit: percentage.
- Status: source-reported actual and target.
- These figures are not route-section capacity, allocated/spare train paths, train-level reliability, customer-level transit time or port dwell.

## 3. Computation

For each metric (m) on corridor (c):

`gap_pp(c,m) = actual_pct(c,m) - target_pct(c,m)`

This is a percentage-point difference, not a relative percentage change.

The CLI reports four independent gaps per corridor:
1. train execution;
2. OTD including 80s;
3. OTD excluding 80s;
4. OTA.

No weighted score or cross-corridor aggregate is computed. The source provides no defensible weights or denominators for such aggregation.

## 4. Validation gates

- Exact header match; unknown schema versions fail closed.
- Exactly six known corridors, each present once.
- No blank records, duplicate corridors, unknown corridor names or malformed columns.
- All percentage fields parse as integers in `0..=100`.
- Targets and actuals remain distinct.
- Derived gaps match hand arithmetic.
- OTD including and excluding 80s remain separate output fields.
- Fixture and manifest source metadata remain adjacent to the data.
- A CSV byte digest in the manifest binds the frozen input; changing the source snapshot requires a new reviewed digest/version.

## 5. South African operational data path

The next source qualification step belongs to [TRANSPORT-DATA-001 #1631](https://github.com/Luminous-Dynamics/symtropy/issues/1631):

- TRIM Network Statement Version 4, published 18 September 2026.
- Annexure 33 Network Register.
- Annexure 33a point-to-point running times.
- Annexure 33b Capacity Statement (updated 29 October 2026 according to the publication page).
- Annexure 43 KPIs.

Sources:
- https://www.transnet.net/TRIM-Networkstatement
- https://www.transnet.net/RenderPage.aspx?id=5371632

These artifacts must be acquired, rights-reviewed, versioned and independently parsed before their values are promoted into scenarios. Annexure 33b is macro-enabled XLSM: treat it only as untrusted data, never execute embedded macros. Preserve theoretical, practical, lost, operational, allocated and spare capacity as distinct fields with the source's units and valid-time scope.

Stats SA P7162 is suitable for aggregate monthly freight-trend plausibility, not direct route-section demand or available-path capacity:
https://www.statssa.gov.za/?PPN=P7162&SCH=74344&page_id=1854

## 6. Comparative programme

Do not compare a new model to a global ranking on a different task. Use matched inputs, known information cutoffs, identical disruption tapes and independently audited feasibility.

### Rail / corridor comparators

1. Historical observed-plan replay, where event-level data and rights permit.
2. Current plan / no-change policy baseline.
3. Simple operational heuristics: FCFS, earliest due date, critical ratio and least slack, adapted to rail path, consist and terminal constraints.
4. Exact enumeration on small finite instances as an oracle.
5. Matched CP-SAT/MILP or other deterministic optimizer.
6. Robust / stochastic rolling-horizon methods with the same forecast horizon and uncertainty inputs.
7. Reinforcement-learning policies only with separated training/validation/test corridors or time blocks, no future-event leakage, and an independently coded safety/feasibility auditor.

The broader Symtropy maritime-service-chain contract (#3683) already defines siloed versus joint planning across vessel, waterway, lock, berth, energy and inland transfer. This scorecard is a measurement input; it does not replace that contract or the transport optimizer (#1601).

### Port / terminal comparators

- World Bank / S&P Global CPPI: comparable vessel time-in-port benchmark; it measures container-ship time waiting and at berth, not end-to-end rail or final-delivery reliability.
- World Bank 2026 Container Terminal Performance Improvement Guide: terminal-level process metrics and simulation/emulation guidance spanning berth, vessel, yard, gate, equipment, labor, maintenance and inland interfaces.
- DCSA Port Call 2.0: operational event-interchange and conformance baseline, not an optimizer.

References:
- https://www.worldbank.org/en/topic/transport/publication/cppi
- https://documents.worldbank.org/en/publication/documents-reports/documentdetail/099103026152033883
- https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-implementation-guide

### Shipbuilding comparators

The public Mendeley *Data of Ship Block Scheduling in English* dataset (DOI 10.17632/2bmbmbf8hm.1, CC BY 4.0 metadata) provides raw block/position inputs and DDQN, EDDQN and five heuristic result families:
https://data.mendeley.com/datasets/2bmbmbf8hm/1

Its stated assumptions include sufficient labour and no absenteeism, compatible positions for differently sized blocks, and initially empty block positions. Reproduce its exact setup before comparing; then add held-out disruption, labour, geometry and equipment-availability scenarios. Existing Symtropy shipyard public-corpus work (#4204) is the owner; do not create a duplicate loader.

### Vessel hydrodynamics and manoeuvring comparators

Use ITTC's 2024 procedure *Benchmark Data for Validation of Manoeuvring Predictions* and the open SIMMAN 2014/2020 cases as validation references for suitable hull/model scopes. A trajectory or service-chain score is not a substitute for validating vessel dynamics.

References:
- https://ittc.info/media/12240/75-02-06-06.pdf
- https://ittc.info/benchmark-repository/manoeuvring-predictions/

## 7. Future measured-performance metrics

Once data rights and source qualification permit event-level evaluation, report at least:

- train plan execution and cancellation reasons;
- OTD/OTA under verified event definitions and denominators;
- median, p90 and p95 port-to-inland and door-to-door transit time;
- vessel waiting, time at berth, container dwell, terminal transfer and rail handoff dwell;
- allocated vs operated train paths and segment bottlenecks;
- cargo tonnes, TEU, gross-tonne-km and empty running;
- customer cost per delivered tonne/TEU, with tariff, market price and social cost separated;
- energy and lifecycle greenhouse-gas emissions per accepted tonne-km;
- service equity across operators, commodity groups and inland destinations;
- disruption recovery time, unmet demand, reroute feasibility and common-cause risk.

Report all objective dimensions separately (Pareto frontiers). Safety, legal, environmental, capacity and evidence-admissibility gates are non-compensable; they cannot be offset by cheaper or faster schedules.

## 8. Proposed advancement gates (not measured results)

- **P0 source reference:** all six values are traceable; validator and arithmetic checks pass.
- **P1 route-data qualification:** source files acquired under reviewed rights; section IDs, units, dates and capacities independently reconciled.
- **P2 historical replay:** event-level baseline reconstructed; no missing/late/future events silently dropped.
- **P3 matched benchmark:** candidate beats or improves the Pareto frontier against the strongest reproducible comparator on held-out periods and disruption classes, with no hard-gate regression.
- **P4 shadow pilot:** operator-approved, read-only comparison on current operations, with no dispatch authority.
- **P5 operational trial:** only under operator, regulator, safety-case and professional approvals, with independent outcome measurement.

No fixed percentage improvement is promised before route, demand and service data are admitted. A credible result must show reproducible improvement over the best matched baseline, not merely a high percentage on this six-row source table.
