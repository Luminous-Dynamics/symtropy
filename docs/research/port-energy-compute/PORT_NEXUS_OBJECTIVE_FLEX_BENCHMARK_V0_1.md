# Port Nexus Objective-Specific Flexibility v0.1

**Protocol:** PORT-NEXUS-OBJECTIVE-FLEX-001  
**Date:** 19 November 2026  
**Status:** synthetic known-answer fixture and benchmark contract; not a production scheduler.

## Why this increment exists

A study available online on 14 November 2026 (Yang et al., *Advances in Applied Energy*, DOI [10.1016/j.adapen.2026.100322](https://doi.org/10.1016/j.adapen.2026.100322)) analyses 600,245 Alibaba AI jobs and finds that power-intensity and cumulative-energy job populations differ. In its scenarios, independently optimizing time-of-use or carbon signals can create new peaks; a no-new-peak admission rule preserves a substantial share of the objective value.

The result changes our design requirement: workload flexibility must be **objective-specific and aggregate-aware**. A single "percent flexible" number hides which jobs are useful for which grid service and whether moving them creates a new system peak.

Caprara et al.'s earlier LAD-Flex study (IJEPES 178, May 2026, DOI [10.1016/j.ijepes.2026.111940](https://doi.org/10.1016/j.ijepes.2026.111940)) is a separate comparator. Do not combine the two papers' results.

## Fixture

`PORT_NEXUS_NO_NEW_PEAK_FIXTURE_V0_1.json` is a deterministic eight-slot synthetic example:
- four one-hour batch jobs, each 2 kW / 2 kWh;
- fixed residual port/facility PCC import profile;
- different TOU price and carbon-intensity signals;
- objective-only greedy placement versus the same policy with a full-horizon no-new-peak admission cap.

The expected outputs show:
- TOU-only shifting creates a 22 kW peak (+22.22%); the guard caps the trajectory at the original 18 kW and retains 78.57% of the objective improvement.
- Carbon-only shifting creates a 23 kW peak (+27.78%); the guard caps the trajectory at 18 kW and retains 55% of the objective improvement.

These exact figures are solely the output of the small synthetic fixture. They do **not** reproduce the November 2026 paper, represent measured port or workload data, demonstrate a global optimum, or establish achievable savings.

## Independent verifier requirements

The first Rust implementation should be an independent admission verifier, not another scheduler:
1. Verify complete job identity/energy conservation and half-open deadline windows.
2. Reconstruct each candidate's aggregate PCC trajectory from the same frozen baseline.
3. Apply full-horizon peak and time-varying PCC envelope checks.
4. Preserve the objective signal identity/vintage and report objective improvement separately from grid admission.
5. Include rebound/recovery in the evaluation horizon.
6. Abstain when the baseline itself violates the declared cap, the objective denominator is zero/negative, source identity differs, or critical fields are missing.
7. Keep per-site, feeder and system constraints separate; a historical site-peak cap is only the first synthetic guard.

The verifier must not claim voltage/frequency stability, N-1 deliverability, system adequacy, market accreditation, or grid-code compliance. Those require separate network, dynamic, adequacy and operator qualification.

## Benchmark sequence

Compare inflexible baseline, FIFO/EDF, LAD-Flex-style deferral, unconstrained TOU, unconstrained carbon, guarded TOU, guarded carbon, PCC-envelope-aware dispatch and constrained MPC/MILP on identical traces and service obligations.

Later evaluation should use the cited public Alibaba trace (after dataset/version/license verification), and measured H100/B200 traces on held-out hardware. Add South African port loads only when rights-cleared, site-authorized time series and actual PCC boundaries are available.

## Project boundaries

- Symtropy: deterministic replay and independent grid-impact admission.
- Symthaea: proposal generation only; it cannot verify its own schedule.
- Sol-Atlas: spatially resolved port/PCC context and source-rights lineage.
- Mycelix: provenance, permission, approval and evidence lifecycle; DHT publication is not physical delivery evidence.

No live controller or port OT interface is changed by this research fixture.


## Research update — port operations and computational-load reliability (19 November 2026)

### A. Port microgrid comparator: event-triggered, logistics-coupled dispatch

Li et al., *Microgrid energy optimal scheduling in ports incorporating the bidirectional event trigger mechanism*, *Journal of Cleaner Production* 568, article 148666 (15 June 2026), DOI [10.1016/j.jclepro.2026.148666](https://doi.org/10.1016/j.jclepro.2026.148666), is now a required comparator.

The Tianjin Dongjiang Port case couples day-ahead berth/quay-crane assignments with renewable use, grid procurement and storage dispatch in an hourly MILP. Its real-time layer uses 15-minute updates, a one-hour rolling horizon and a bidirectional event trigger that watches both logistics-side and energy-side deviations. The paper reports operating-cost reductions of 18.64% and 22.44% versus its MPC and logistics-event-trigger baselines, respectively, and online solution-time reductions of 88.90% and 77.76% in that case.

This is a stronger port baseline than a generic rule-based EMS alone. It also identifies a key weakness in simple port-energy models: vessel/berth/logistics operations are often treated as fixed external loads, even though berth allocation and crane schedules create shore-power and cargo-handling demand.

**Required benchmark comparison:** include a port-logistics-aware day-ahead MILP + 15-minute event-triggered re-optimization baseline, with the same berth, vessel, tariff, weather, renewable, storage and load inputs. Do not claim a cost or solver-time advantage until the actual paper's model/data are reproduced or a clearly matched new case is run.

Our candidate extension should introduce compute-job selection and deadlines into this event-triggered port system, while preserving aggregate PCC/feeder/ramp admission. It should trigger re-optimization only when a material logistics, energy, compute, grid or marine-state deviation is detected. The trigger must be value-of-information / constraint-margin aware, not an arbitrary fixed timer.

Source: [ScienceDirect — BETM port microgrid scheduling](https://www.sciencedirect.com/science/article/pii/S0959652626012072).

### B. New grid-reliability reference: NERC computational-load standards

The U.S. North American Electric Reliability Corporation (NERC) Board adopted the first three Computational Load Ownership & Operations (CLO) standards on 26 October 2026. NERC submitted them to FERC on **16 November 2026**; the standards pages currently identify the filing as pending regulatory filing.

The three proposed standards create a useful engineering crosswalk:
- **CLO-001-1:** computational-load interconnection requirements, studies and modelling data;
- **CLO-002-1:** operational data, communications and response to operating instructions;
- **CLO-003-1:** protection coordination and disturbance monitoring.

This is not an approved U.S. regulatory obligation yet, and it is not a South African grid-code requirement. Treat it as a current industry direction and a design-reference profile. The programme should maintain a separate South African/NTCSA requirements mapping for any local project.

The port-compute evidence package should therefore carry:
1. interconnection model inputs, versions, topology and model-validation evidence;
2. site operational data and communications readiness;
3. high-resolution disturbance observations and protection-coordination references;
4. clear site/operator identity and operating-instruction handling;
5. exact claim boundaries between software demand response, involuntary disturbance trips, and physical grid-support services.

Sources: [NERC Large Loads Action Plan — 16 November 2026 filing](https://www.nerc.com/initiatives/large-loads-action-plan); [NERC CLO standards status](https://www.nerc.com/standards/reliability-standards/clo); [NERC standards submission coverage, 17 November 2026](https://www.publicpower.org/periodical/article/nerc-seeks-ferc-approval-proposed-reliability-standards-tied-integration-computational-loads).

### C. Port environmental-data gap: benchmark outcomes, not disclosure volume

Alavi-Borazjani et al., *Environmental KPI Reporting in Ports: Patterns, Gaps, and Opportunities for Systemic Change*, *International Journal of Environmental Research* 20, article 166 (25 May 2026), DOI [10.1007/s41742-026-01130-4](https://doi.org/10.1007/s41742-026-01130-4), benchmarks 403 environmental KPIs disclosed across 25 ports and finds fragmented, uneven reporting. It identifies under-representation of outcome-oriented and integrated indicators, including weak tracking of progress toward reduction targets, climate resilience, acoustic impact and circularity.

This reinforces the Sol-Atlas/Mycelix opportunity: publish a source-reversible, outcome-oriented port evidence model rather than another ESG dashboard. Each KPI must have a definition, owner, measurement boundary, source/time, unit, uncertainty/calibration state, rights, target/counterfactual and correction history. Distinguish installed capacity and declared plans from delivered outcomes.

Source: [Springer Nature — Environmental KPI Reporting in Ports (2026)](https://link.springer.com/article/10.1007/s41742-026-01130-4).

### D. Underwater comparator: distinguish operating phase from planned build-out

Shanghai Lingang's underwater data-centre project entered operation in May 2026, but the publicly described initial operating phase is **2.3 MW** while **24 MW is the planned overall capacity**. Shanghai's June report also describes direct offshore-wind connection and seawater heat exchange, with source-reported 22.8% electricity-consumption reduction and zero freshwater use. These are project/operator or government-reported figures; no independent lifecycle validation is asserted here.

Our comparator must not treat planned 24 MW as operating capacity, and must match the metering boundary, IT utilisation, weather, water boundary and lifecycle scope before comparing PUE, electricity use, freshwater or carbon.

Sources: [Shanghai Government / China Daily, 1 June 2026](https://english.shanghai.gov.cn/en-Latest-WhatsNew/20260601/89a94ac072f842eeacf82fcf0cce8261.html); [Shanghai project launch and Phase 1 description](https://english.shanghai.gov.cn/en-Strength-Scitechcenter/20250611/c60460fe62e440d08b62be991cade0c9.html).

## New research priorities after this update

1. **Co-schedule port logistics, shore power and compute.** Berth allocation, vessel call uncertainty, shore-power connection, crane/yard equipment and compute workloads must share one time-indexed service model. Start with day-ahead hourly plans and event-triggered 15-minute correction; preserve a separate high-frequency local-control boundary.
2. **Trigger only when the state warrants it.** Combine logistics deviation, load forecast error, renewable deviation, PCC margin, compute queue/deadline slack and marine/weather state. Measure avoided solver calls against stale-plan risk and objective loss.
3. **Add a constraint-margin budget.** A candidate may not spend the same headroom twice across shore power, compute, storage charge, cooling and export commitments. Every resource reservation is tied to the same PCC and feeder envelope.
4. **Report outcome evidence.** Measure actual vessel auxiliary energy avoided, on-time shore-power delivery, port-service completion, compute useful work, grid impact, marine change and community outcomes—not just planned MW or installed assets.
5. **Separate proposed standards from approved obligations.** Maintain explicit jurisdiction and status for each requirement. NERC CLO is a U.S. pending filing as of this snapshot; it is not a South African approval or certification.

The new Rust verifier in `tools/port-nexus-flex-verifier/` covers only schedule identity, whole-horizon PCC no-new-peak, declared PCC import/ramp and declared feeder import envelopes. It is the first admission layer, not the coupled logistics/energy/compute optimiser, a network power-flow solver or a standards-conformity assessment.
