# Industry Comparator Register — Maritime, Rail and Shipyard Systems

**Research snapshot:** 21 November 2026  
**Purpose:** identify valid comparison families and public references. Inclusion does not mean the published model is independently reproduced, production deployed, or universally best.

## 1. Port and terminal operations

### Industry measurement layer

| Reference | What it measures or standardizes | How we should use it | What it does not prove |
|---|---|---|---|
| [World Bank / S&P Global CPPI 2025](https://www.worldbank.org/en/topic/transport/publication/cppi) | Comparable container-ship time in port, including waiting and berth time | Port-level external comparison and multi-year trend | Rail dispatch, inland delivery, attribution to one operator, or terminal-only performance |
| [World Bank 2026 Container Terminal Performance Improvement Guide](https://documents.worldbank.org/en/publication/documents-reports/documentdetail/099103026152033883) | Terminal process benchmarks across berth, vessel, yard, gate, equipment, labour, maintenance and interfaces | Diagnose where time is consumed; compare terminals with similar traffic and operating profiles | A universal target or an operator-only attribution for port-wide time |
| [DCSA Port Call 2.0](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-implementation-guide) | Interoperable port-call event exchange and conformance | Validate event semantics, publisher/subscriber behaviour and common operational picture | A berth optimizer, resource commitment, or operational authority |

The 2025 CPPI-derived 2026 Guide reports descriptive overall averages of 44.8 container moves per port hour, 56.3 per berth hour, 23.5 gross ship-to-shore moves per crane-hour, 12.3 hours waiting and 16.4 hours arrival time (waiting plus steam-in). These are descriptive arithmetic means, not targets. Compare like-for-like terminal/call profiles and respect the metric boundary; several vessel-call measures include actors outside the terminal operator.

### Recent research frontier

| Method family / publication | Comparator to reproduce |
|---|---|
| Predict-then-optimize JIT arrival with probabilistic berth-availability forecasts, *Transportation Research Part E* (2026), [DOI 10.1016/j.tre.2026.105033](https://doi.org/10.1016/j.tre.2026.105033) | Baseline ETA vs calibrated arrival distribution; optimize speed/arrival under the predicted berth-availability distribution; score using realized berth and arrival times |
| Integrated berth allocation and time-varying quay-crane scheduling with reinforcement learning + adaptive large-neighbourhood search, *Ocean Engineering* (2026), [DOI 10.1016/j.oceaneng.2026.124721](https://doi.org/10.1016/j.oceaneng.2026.124721) | Reproduce the same BAP/QCSP formulation, arrival and processing assumptions, objective and compute budget; compare RL+ALNS with exact and heuristic baselines |
| Two-stage robust integrated berth allocation, quay-crane assignment and yard assignment, *Computers & Industrial Engineering* (2026), [DOI 10.1016/j.cie.2026.112092](https://doi.org/10.1016/j.cie.2026.112092) | Compare deterministic plan, two-stage robust plan and column/constraint-generation-based heuristic under identical uncertainty sets and recourse rules |
| Joint vessel traffic scheduling and berth allocation for multi-basin / multi-anchorage ports with a one-way channel, *Ocean Engineering* (2026), [DOI 10.1016/j.oceaneng.2026.128093](https://doi.org/10.1016/j.oceaneng.2026.128093) | Compare unified inbound/outbound sequencing and berth allocation with siloed schedules; preserve channel conflicts, safety headways and tidal windows |

A 2025 peer-reviewed review of 94 berth-allocation and quay-crane scheduling papers finds continued use of MILP, metaheuristics, machine learning/reinforcement learning, stochastic and robust optimization, with growing emphasis on integrated and continuous berthing. It also notes that many studies prioritize efficiency while leaving policy trade-offs less developed: [Makhado et al., *Journal of Marine Science and Engineering* 13(7), 1339](https://doi.org/10.3390/jmse13071339).

## 2. Freight rail and disruption recovery

| Reference | Comparator to reproduce | Required discipline |
|---|---|---|
| Dynamic train timetable rescheduling plus rolling-stock reassignment using improved Benders decomposition, *Computers & Industrial Engineering* (March 2026), [DOI 10.1016/j.cie.2025.111794](https://doi.org/10.1016/j.cie.2025.111794) | Joint timetable and rolling-stock recovery under uncertain disruption duration | Same initial timetable, stock pool, disruption tape, recovery horizon and delay/cost objective |
| Robust rescheduling of train timetables and maintenance windows with distributionally robust optimization and generalized branch-and-Benders-cut, *Transportation Research Part C* (September 2026), [DOI 10.1016/j.trc.2026.105781](https://doi.org/10.1016/j.trc.2026.105781) | Robust schedule and maintenance-window recovery under uncertain disruption duration | Same ambiguity set, forecast information, maintenance constraints and computation budget |
| [TRIM Network Statement Version 4](https://www.transnet.net/TRIM-Networkstatement) | South African section-level infrastructure and path context | Qualify theoretical, practical, lost, operational, allocated and spare capacity separately; bind version, direction, valid time and units |

For South Africa, global rail papers are algorithmic comparators, not direct performance comparators. The network's gauge, gradients, traction, train lengths, axle loads, passing loops, signalling, operational rules, terminal constraints, maintenance state and available evidence must be reflected in a local benchmark before comparing results.

## 3. Shipyard and marine manufacturing

### Public block-scheduling dataset

[Data of Ship Block Scheduling in English](https://data.mendeley.com/datasets/2bmbmbf8hm/1), DOI [10.17632/2bmbmbf8hm.1](https://doi.org/10.17632/2bmbmbf8hm.1), was published 3 March 2026 and reports CC BY 4.0 metadata. It supplies raw block and position inputs plus DDQN, EDDQN and five heuristic result families.

The published setup assumes sufficient labour without absenteeism, each block is completed at a single block position, positions begin empty, and different-sized blocks can find compatible positions. These assumptions make it a reproducibility baseline, not a robust shipyard digital twin.

Compare:
- exact published DDQN / EDDQN / heuristic results after acquiring and hashing the original files;
- earliest-start, longest-processing-time, resource-utilisation, response-time and shortest-processing-time heuristics;
- exact enumeration on small instances;
- mixed-integer or CP-SAT baselines where the formulation permits;
- disruption- and geometry-aware extension with labour absenteeism, crane/lift limits, equipment downtime, workpiece precedence, rework and uncertainty.

Keep the published-data reproduction split separate from the new disruption/generalization evaluation.

### Fabrication and welding

For welding, compare qualified process windows and measured weld outcomes, not only planned robot trajectories. The Symtropy shipyard work already contains distinct welding, metrology and digital-twin commissioning efforts (including #4435 and #4841); this register does not create another weld model or process authority.

## 4. Vessel dynamics and hydrodynamic validation

Use ITTC's 2024 *Benchmark Data for Validation of Manoeuvring Predictions* and SIMMAN 2014/2020 open benchmark cases: [ITTC procedure 7.5-02-06-06](https://ittc.info/media/12240/75-02-06-06.pdf) and [ITTC manoeuvring benchmark repository](https://ittc.info/benchmark-repository/manoeuvring-predictions/).

For relevant hulls and test envelopes, validate against measured turning-circle, zig-zag and captive-test observations with documented hull particulars, coordinate systems, loading, rudder rate, water depth and uncertainty. Do not treat a scheduler's arrival feasibility as proof of navigational safety, or a generic CFD residual as physical validation.

## 5. How to compare fairly

Every algorithm comparison should freeze:

1. **Problem definition:** same topology, demands, vessel/train/yard capabilities, hard constraints and lifecycle boundaries.
2. **Information set:** identical forecast cut-off; no future-event leakage; no post-event observations available to an earlier decision.
3. **Disturbances:** paired immutable event tapes, including outages, weather, equipment failure, late arrivals, labour absence and data-quality faults.
4. **Compute budget:** same hardware class, wall-clock budget, time limit, memory limit and stopping rule. Include preprocessing and re-optimization costs.
5. **Feasibility:** an independent auditor that does not share the candidate's constraint helpers. Invalid plans cannot earn a lower objective score.
6. **Evaluation:** held-out episodes/time blocks/locations, multiple seeds where stochastic, confidence intervals, full Pareto vectors and all failed/time-out/abstained cases retained.
7. **Reproduction:** exact input digests, code revision, model configuration, solver versions, run receipt and independent replay.

Do not compare a synthetic pass rate to CPPI, an RL paper's reported percentage to an unrelated industrial KPI, or a simulator makespan to actual door-to-door service.

## 6. Proposed target ladder (research targets, not observed results)

| Gate | Target |
|---|---|
| Source correctness | 100% of admitted fixture rows trace to the cited source; 100% digest match; zero unknowns silently converted to zero |
| Small-instance correctness | Match the independent exact oracle on every tractable frozen instance |
| Hard constraints | Zero capacity, precedence, compatibility, safety-envelope or rights violations in qualified candidate schedules |
| Generalization | No regression hidden by aggregate averages: publish per-corridor / per-terminal / per-scenario and worst-decile outcomes |
| Prediction | Calibrated prediction intervals on temporally and geographically held-out data; report coverage and sharpness together |
| Operational value | Demonstrate a reproducible Pareto improvement over the strongest matched baseline: e.g. lower p90 delivered delay or lifecycle cost without worse safety, service reliability, emissions or distributional outcomes |
| Resilience | Lower unmet demand and recovery time under paired disruptions, with no increase in unauthorized commitments or unreconciled state |
| Compute | Publish quality-vs-time curves and p50/p95 runtime on declared hardware; no universal time target until a representative South African workload is admitted |

The stretch goal is not "beat one paper by 5%". It is to beat or match strong exact/robust/hybrid baselines on the same problem, then show better held-out performance when source quality shifts, disruptions correlate, operators differ, and downstream handoffs matter.

## 7. Existing Luminous work to compose

### Cross-stack owners
- Mycelix #5968 — Maritime Commons cross-stack integration architecture.
- Mycelix #5974 / draft PR #5980, stacked on draft PR #5972 — 35-case four-owner golden trace and independent oracle. The declared owner outputs are synthetic fixtures; this is not yet proof of four live production systems integrating.
- Mycelix #5920 / #5185 / #5963 — federated provider/resource authority, participant-authorized schedule flexibility and adapter semantic fidelity.
- Sol-Atlas #654 / #115 — source-reversible spatial/temporal maritime evidence projection.
- Sol-Atlas #377 / #380 — coast–hinterland opportunity discovery and ex-post port/corridor outcome evaluation.
- Symthaea #7444 / draft PR #7629 — evidence-bounded marine freight screening and interval qualification. This is not a naval-architecture solver or physical vessel approval.
- Symthaea draft PR #7705 and #11080 — concept-stage marine vessel families and mass/loading-condition evidence.

### Symtropy operational, physical and manufacturing owners
- Symtropy #1601 / PR #1602 — multimodal whole-life optimization and modular cargo.
- Symtropy #1631 / #3243 — South African route/capacity source qualification and Southern African freight baseline.
- Symtropy #1858 / PR #1859 — JIT arrival and port energy.
- Symtropy #1867 / PR #1868 — contingent recovery options.
- Symtropy #3683 / PR #3705 — coupled maritime service-chain contract.
- Symtropy #3865 — physically qualified ship-handling envelopes.
- Symtropy #4204 — ship-block public-corpus admission.
- Symtropy #4435 / #4841 — shipyard welding and metrology.
- Symtropy #6100 — shipyard space-time feasibility.
- Symtropy #6257 / #6277 — multi-rate port event monitoring and port OT cyber trust.
- Symtropy #6473 / PR #6474 — this aggregate South African corridor reference slice.

The research PRs above are draft work unless their individual status and exact-head checks say otherwise. This register is a comparator map, not a claim that the full Luminous stack is already integrated or operational.
