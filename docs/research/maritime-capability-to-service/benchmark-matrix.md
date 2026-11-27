# Maritime Industrial Benchmark Matrix v0.1

Companion to [MCSC #7686](https://github.com/Luminous-Dynamics/symtropy/issues/7686). This is a benchmark protocol, not a claim that any target has been achieved.

## 1. Adopt the industry's shared language; differentiate above it

| Reference | What it standardizes/measures | Programme treatment |
|---|---|---|
| [DCSA Port Call 2.0.0](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope) | Standardized operational information exchange among carriers, terminals, ports and service providers. Its interface scope focuses on port-call planning through berth arrival; cargo operations are separate parallel processes. | Implement/adopt the standard and its conformance scenarios. Do not invent a second port-call API or claim that port-call conformance proves cargo delivery. |
| [IAPH/IHMA Port Call Optimization Guide](https://www.iaphworldports.org/pickup/21900/) | A minimum high-value nautical/operational data set, including terminal/berth identity and key planned times, with port-to-port trials. | Consume its semantics and identifiers. Differentiate through resource feasibility, authority-preserving replanning and measured outcomes. |
| [World Bank / S&P CPPI methodology](https://thedocs.worldbank.org/en/doc/aac122f6df85534428d66a7b9af4b7f6-0400012026/original/CPPI-Methodology-Note.pdf) | Port-level vessel-time index based on six port-call milestones and carrier records matched with AIS. | Preserve a CPPI-compatible port-time scorecard where the source data supports it. Add receiver-accepted service as a separate outcome, not a replacement score. |

### CPPI-compatible metric boundary

CPPI's six timestamps are: arrival at anchorage/pilot station; movement to berth; start of cargo operations; end of cargo operations; departure from berth; exit from port limits. CPPI is a port-level benchmark, not an individual terminal audit. Do not fold receiver acceptance, inland delivery, post-berth repair or lifecycle maintenance into the CPPI number. Report those separately with their own denominators, sources and acceptance rules.

DCSA's current implementation guidance also points adopters to its conformance framework and sandbox. We should use the conformance scenarios as the interop acceptance gate; an internally successful adapter is not proof of DCSA conformance.

## 2. Manufacturing intelligence and digital-twin assurance

[NIST SI for Manufacturing](https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing) is a measurement-science comparator, not a product competitor. Its current research threads include trustworthy manufacturing intelligence, human-machine teaming, contextualized manufacturing data, integration test benches/benchmark studies and digital-twin verification, validation and uncertainty quantification (VVUQ). Its TIDAL framing connects asset-, site- and enterprise-level intelligence with human roles, operational systems and digital-twin foundations.

Programme requirement:
- Treat NIST's evaluation methods, metrics, datasets, test-bench concepts and standards work as a design constraint for our benchmark protocol.
- Provide an integration seam with owner and authority for every decision, source and observation.
- Separate model-predicted state, observed physical state, accepted output and human authorization.
- Calibrate model fidelity to the decision: no digital-twin claim without a declared validation domain, uncertainty budget and measurement trace.
- Compare human-machine teaming on inspectable work products, correction rate, safe overrides, time-to-competence and responsibility clarity—not only model task completion.

## 3. Published port-energy comparators

These results show why matched reproduction must precede any superiority claim. They are reported case-study outcomes, not universal port performance limits and not directly comparable to one another.

| Published method | Reported result in its study | Required reproduction |
|---|---|---|
| Li et al., *Microgrid energy optimal scheduling in ports incorporating the bidirectional event trigger mechanism*, Journal of Cleaner Production 568 (15 Jun 2026), DOI [10.1016/j.jclepro.2026.148666](https://doi.org/10.1016/j.jclepro.2026.148666) | Tianjin Dongjiang case: operating cost reduced 18.64% vs MPC and 22.44% vs a logistics event-triggered approach; reported online-solve-time reductions 88.90% and 77.76%; near-zero renewable curtailment. | Reproduce the day-ahead/15-minute real-time cadence, 1-hour reoptimization horizon, event triggers, forecast deviations, storage, tariffs and exact baselines. Match all input traces and solver budgets before comparing. |
| Li & Hu, *Integrated optimization of berth allocation, crane assignment, and onshore power supply with multi-source energy dispatch for green ports*, Journal of Cleaner Production 574 (18 Aug 2026), DOI [10.1016/j.jclepro.2026.149105](https://doi.org/10.1016/j.jclepro.2026.149105) | Integrated berth, crane, shore-power and energy dispatch; reports cost reductions up to 19.87% in its cases. | Match berth/crane/OPS decisions, energy sources, power tracing, carbon accounting, MILP linearization, constraints and all objective weights. |
| Li & Hu, *Collaborative optimization of berth scheduling and multi-energy dispatch for green ports*, Omega 143 (Sep 2026), DOI [10.1016/j.omega.2026.103549](https://doi.org/10.1016/j.omega.2026.103549) | Joint berth and multi-energy dispatch; models logistics-driven load response, renewables, CCHP, electrical/thermal storage and bidirectional grid trading. | Reconstruct formulation, demand profiles, energy prices, renewable profiles, storage and network constraints before using it as a baseline. |

### Mandatory parity sheet for every paper reproduction

Freeze and publish: original source version; all data/input files and checksums; time horizon and discretization; vessel/cargo set; berth/crane/AGV/yard/OPS decisions; power/heat/fuel sources; tariff/carbon assumptions; operational constraints; uncertainty set and distribution; objective formula and units; solver/version; hardware; thread count; time limit; warm start; stopping tolerance; best bound/gap; random seeds; and exact baseline implementation.

If any item cannot be recovered, label the result a partial reconstruction, not a faithful reproduction. A 20% result on a different problem is not evidence of beating a paper's 19% result.

## 4. Shipyard scheduling baseline

[Symtropy #4204](https://github.com/Luminous-Dynamics/symtropy/issues/4204) owns the English/Chinese Mendeley ship-block data admission and faithful result reproduction. The English dataset record is [Mendeley Data, DOI 10.17632/2bmbmbf8hm.1](https://doi.org/10.17632/2bmbmbf8hm.1), metadata published 3 March 2026 and licensed CC BY 4.0. It contains block and block-position input data and result files for DDQN, EDDQN and five heuristics; the source says the data is adapted from real-world shipyard production data with anonymity/experimental adjustments.

The source assumptions include:
- minimum-bounding-rectangle representation;
- one block per position at a time;
- sufficient labour, with no absenteeism/labour anomalies;
- a suitable position assumed to exist for each block;
- each block completed at one position;
- all block positions empty at schedule start.

Those assumptions must be preserved for Layer A faithful reproduction. Only after Layer A and independent feasibility reconstruction pass should we run Layer C controlled extensions: non-empty initial occupancy, labour absenteeism/skills/shifts, lift/crane capacity, transport fleets, inspection holds, rework, material delay and machine outage. Each extension receives a distinct scenario-family label and is not mixed into the published baseline score.

## 5. End-to-end service and productive capability

Consume the canonical owners:
- Mycelix #6773: regional maritime productive capability and time-windowed qualification.
- Mycelix #4814 and #5762: supplier/resource and common-cause dependency semantics.
- Symtropy #3683: expected-service denominator, cargo disposition and receiver acceptance.
- Symtropy #4204: public ship-block data admission and faithful scheduling reproduction.
- Symtropy #2048/#2050/#2734: build, spatial feasibility and non-anticipative recovery owners.
- Sol-Atlas #370/#377/#948: spatial, rights-aware and site evidence.

The MCSC contribution is only the counterfactual join: hold the service obligation and exogenous shocks fixed, vary the qualified capability portfolio, and evaluate the difference in receiver-delivered service and recovery. No second dependency graph, yard scheduler or cargo ledger.

### Required outcome vector (publish each component)

1. **Receiver service:** expected units, accepted units, on-time accepted units, partial units, late units, diverted/cancelled units, unresolved/unknown units; receiver elapsed-time percentiles calculated only over the declared eligible set.
2. **Port/asset operation:** CPPI-compatible six-milestone times; berth/vessel wait; cargo operations and equipment utilization; shore-power service; power peak/curtailment; service shortfall.
3. **Productive capability:** exact qualified grades/processes/tolerances/volume; capacity by time window; first-pass yield; inspection queue/rework; qualified repair and substitute activation time.
4. **Resilience:** direct and transitive shared dependencies; minimal cut sets under the declared fault model; accepted-service loss under common-cause shocks; time to minimum viable service and full recovery.
5. **Economics and distribution:** whole-life cost per accepted unit; gross local spend, local value added and retained capability separately; working capital; training vs qualified competency; supplier concentration.
6. **Physical/environmental:** mass and energy balances; grid/renewable/fuel/water/heat use; material yield, waste and embodied/operational emissions with declared boundaries.
7. **Evidence quality:** provenance completeness, freshness, measurement uncertainty, model VVUQ coverage, human approvals, invalidated assumptions and unresolved evidence.

Do not collapse these into one weighted scalar. Preserve hard gates, service floors, Pareto alternatives and distributional floors; publish trade-offs and dominated portfolios separately.

## 6. Benchmark ladder and targets

Targets below are proposed qualification thresholds, not achieved results.

| Stage | Minimum evidence | Gate |
|---|---|---|
| 0. Deterministic micro-oracle | 10 independently adjudicated tiny instances; all finite alternatives enumerated | 100% outcome/oracle agreement; zero false feasibility/acceptance admissions |
| 1. Frozen corpus | 30 cases minimum, expanding to 100+; exact source and fixture digests | 100% inputs accounted for; no silently dropped units, options, failed runs or constraints |
| 2. Matched reproductions | At least three strong baselines across port-energy, shipyard scheduling and service orchestration | Reproduce source formulation, data, objective, units, hardware/compute limits and bounds; disclose all mismatches |
| 3. Robust generalization | Hold out full facilities, shock families, arrival regimes and supplier topologies | No future-information leakage; report paired results, confidence intervals, failure cases and timeouts |
| 4. Physical/industrial validation | Controlled hardware/operational data with rights, safety and authority approvals | Independent VVUQ, calibrated sensors, measured acceptance, traceable intervention and rollback |
| 5. External qualification | Independent evaluator and reproducible public report | Third-party replication; never infer generality from one demonstrator |

### Specific proposed targets

- **Hard-gate integrity:** zero false admissions on all frozen invalid/stale/revoked/unqualified dependency controls; zero unknown or late units relabelled as receiver-accepted.
- **Exactness:** 100% agreement on tiny exact cases across two separately implemented evaluators.
- **Corpus:** at least 30 frozen cases and 10 independently adjudicated tiny cases for initial qualification; 100+ cases for broader screening. Count scenario families and facilities, not merely random seeds.
- **Optimization:** report best-known feasible and best bound, optimality gap, runtime, memory, timeouts and compute. A proposed ≤5% gap applies only where a valid bound is available; it is not a claim of global optimality.
- **Held-out improvement:** only after faithful reproduction, target ≥5% improvement on a predeclared service/cost/recovery measure over the strongest matched baseline, with paired uncertainty reported and no worsening of protected service/safety/environmental floors. Require improvement on more than one held-out scenario family. If the strongest baseline wins, publish that.
- **Operational assurance:** all decisions have authority, timestamped source state, declared validity, constraints, rejected alternatives and a replayable receipt.

Do not set a numeric target against the 18–22% port-energy results until those specific studies have been faithfully reproduced. The first win is a correct, matched and independently replayable baseline—not a flattering number.

## 7. Paradigm-shift hypothesis

The research hypothesis to test—not assume—is that **regional productive capability has marginal service value only when its complete qualified path is independent enough, available in the required time window, and connected to an accepted receiver outcome.**

That reframes local industrial policy from an expenditure quota into a portfolio of service-capable options, and reframes maritime optimization from isolated asset efficiency into whole-chain capability formation, operation and recovery.

A synthetic PASS validates only the frozen model and reference calculations. It does not prove actual regional industrial capability, economic additionality, employment impact, physical safety, port performance or causal impact.
