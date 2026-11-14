# EV engineering benchmark programme: evidence, service and lifecycle

## Thesis

Optimize the delivery of useful service over the full lifecycle, not a component's peak metric. Treat vehicle, battery, charger, site grid, fleet schedule, repair operation and material recovery as a coupled but independently verifiable system.

## Paradigm shifts

1. **From vehicle-centric to service-centric design.** Use passenger-km, trips served, tonne-km, work cycles, hectares serviced or m³ moved as functional units.
2. **From peak charging power to deadline-feasible energy delivery.** Measure delivered energy, thermal derating, queueing, departure success, site limits and repeated-session behavior.
3. **From nominal battery cost to lifecycle service cost.** Include repair granularity, parts availability, downtime, replacements, energy and end-of-life pathways.
4. **From monolithic optimizer to independent execution-feasibility oracle.** An energy-balanced schedule is not valid if equipment ramp rates, connector limits, thermal constraints, duty-cycle timing or grid limits make it impossible.
5. **From a single score to constrained Pareto evidence.** Safety, legal requirements and mission feasibility are hard gates; they cannot be offset by lower cost or carbon.
6. **From claims to reproducible evidence.** Keep synthetic results, manufacturer claims, published measurements and independently measured results distinct.

## Research basis

- The US Joint Office describes charging reliability as an end-to-end ecosystem problem involving standards, interoperability, user experience, metrology and reliability: https://driveelectric.gov/standards-reliability
- SAE J3271 (issued 2025) addresses megawatt charging couplers, communications, cable cooling/handling, grid interconnection and bidirectional use cases: https://saemobilus.sae.org/standards/j3271_202503-sae-megawatt-charging-system-electric-vehicles
- IEC 61851-23-3:2026 specifies MCS DC supply equipment within its defined voltage envelope; it does not by itself qualify every vehicle/site combination or cover all maintenance safety: https://webstore.ansi.org/standards/iec/IEC6185123Eden2026-2617387
- A 2026 parametric LCA study reports that pack integration, compression strategy, disassembly and allocation choices can materially change modeled lifecycle results; its reported ranges are specific to its models and assumptions: https://doi.org/10.1016/j.resconrec.2026.108930
- A 2026 design-for-disassembly study illustrates trade-offs between production/use impacts and repair/reuse outcomes for electric two-wheeler packs: https://doi.org/10.1016/j.spc.2026.04.003
- 2026 ultra-fast charging announcements highlight the need to separate peak/vendor claims from independently measured charge curves and repeated-session degradation: https://www.reuters.com/world/asia-pacific/chinas-high-speed-race-fast-charging-evs-2026-09-24/

## Benchmark tracks

### A. Mission service
- Primary: percentage of requested missions completed on time and within service quality.
- Secondary: useful work per unit energy, unserved work, reserve margin and recovery after disruption.
- Compare identical duty cycles, weather, payload, routes and service boundaries.

### B. Charging and fleet control
- Session start success, delivered energy, average/peak power, p50/p95 queue wait, departure SOC shortfall, missed departures, site peak import, energy losses, charger utilization and battery-throughput proxy.
- Stress arrival bursts, charger failure, feeder caps, weak-grid conditions, hot/cold ambient, missing telemetry and forecast errors.
- Baselines: FCFS/greedy; a transparent schedule heuristic; constrained optimization; proposed controller.

### C. Battery lifecycle and repair
- Wh/kg usable, kWh and useful service over life, measured repair-time distribution, parts wait, return-to-service time, pack replacement count, declared lifecycle cost and GWP, recovered material by stream, contamination and residual mass.
- Include battery aging and uncertainty only when traceable data and model validation exist.
- Require independent structural, electrical, thermal, diagnostic, repair and end-of-life qualification evidence.

### D. Grid and energy architecture
- Minimize required import capacity and total system cost while satisfying mission deadlines and reserve requirements.
- Compare direct grid, managed charging, storage-buffered charging, solar-plus-storage, vehicle-to-grid where enabled and operationally supported, and demand-flexible scheduling.
- Account for round-trip losses, battery wear, export restrictions, tariffs, outage behavior and backup capacity.

### E. Interoperability and trust
- Success rate across supported charger/vehicle/protocol combinations; energy-metering error; payment/authentication recovery; safe shutdown and restart; firmware/configuration provenance; rollback and communication-loss behavior.
- Use current applicable standards and accredited test methods; do not infer compliance from software simulation.

## Software acceptance targets

- Zero silently accepted invalid inputs or unknown configuration fields.
- Canonical reports invariant across 100 input-order permutations.
- Detect all deliberately injected hard-constraint violations in the authored negative-control suite.
- Zero candidates marked qualified while mandatory hard gates are unknown or failed.
- Preserve all failed scenarios; no cherry-picked removal from aggregate results.
- Every reported result identifies source inputs, model version, solver settings, configuration, evidence class, units and lifecycle boundary.
- Report distributions and uncertainty when repeat samples exist. Mark unavailable metrics explicitly for synthetic or single-sample cases.

## Physical benchmarking protocol

1. Freeze protocol and functional unit before running the comparison.
2. Measure baseline first; document instrumentation, calibration, ambient conditions, configuration and uncertainty.
3. Run matched baseline/candidate trials and publish all valid runs, including failures.
4. Choose sample size and minimum practically important effect using baseline variance, not an arbitrary headline percentage.
5. Separate vendor claims, paper results, simulation, field data and independent lab tests.
6. Re-run after material model, firmware, hardware or input-data changes.

## What would count as beating the state of the art?

A preregistered, independently reproducible improvement on a meaningful service metric, with uncertainty intervals, under matched constraints and multiple representative stress scenarios—without regressing safety, durability, accessibility, repairability or lifecycle boundaries. A simulation-only improvement is a model result, not proof of physical superiority.

## Local/industrial priority

Start with rural and industrial fleet electrification: predictable missions, scarce grid capacity, high downtime costs, thermal stress and costly charging delays make service-oriented design especially valuable. Use South African duty cycles, tariffs, ambient conditions and grid constraints only when actual sources are available; don't assume one city's conditions represent every site.

## Non-goals and claim limits

This programme is not vehicle certification, a validated battery electrochemistry model, a thermal runaway/crash test, accredited metrology, a full ISO-conformant LCA, or proof of production readiness. Every physical claim requires appropriate evidence and expert review.