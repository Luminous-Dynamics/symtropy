# Polar resilience benchmark targets and industry-comparison protocol

**Status: proposed research protocol. Targets below are hypotheses, not achieved results.**  
**Current evidence ceiling: synthetic scorer correctness only.** This crate scores supplied traces; it does not simulate electrical/thermal dynamics, optimize dispatch, certify a controller, or validate an Arctic/Antarctic site.

## Why this benchmark should differ

A conventional asset-availability or energy-cost metric can look good while people lose heat, water, sanitation, or communications. Evaluate delivered services over time against owner-approved floors; report fuel, cost, repair burden, compute, and other trade-offs separately. Lower fuel use never compensates for a breached critical-service floor.

Distinguish two questions. Where credible scenario probabilities exist, use probability-weighted metrics and tail measures with uncertainty. Where probabilities are not defensible, report each enumerated stress scenario and worst-case outcomes; do not relabel an unweighted stress suite as expected risk. This reflects recent microgrid reliability research emphasizing chronological adequacy and correlated scarcity rather than a single headline indicator. HIL, commissioning, and periodic controller testing are separate evidence stages, not certificates implied by a passing unit test.

## Comparison arms

Use identical episode boundaries, service contracts, input data, constraints, hardware, and reporting rules. Pre-register tuning budgets and freeze the final hold-out set.

| Arm | Purpose | Fairness requirement |
|---|---|---|
| Documented incumbent/operator policy | Real operational reference | Version the policy and record operator overrides |
| Transparent rule-based policy | Reproducible low-complexity baseline | Publish thresholds, priorities, fallback behavior |
| LP/MILP planner, when the model supports it | Optimization baseline | Same asset limits, forecasts, horizon, input information |
| MPC/receding-horizon optimizer, when feasible | Test value of replanning | Match decision deadlines and forecast access |
| Robust/scenario optimizer, when uncertainty is modeled | Uncertainty-aware comparator | Same stress cases and documented uncertainty set |
| Proposed controller/planner | Candidate | Freeze the candidate before final evaluation |
| Negative controls | Demonstrate fault detection | Floor violator, failed actuator, stale sensor, invalid evidence |

Do not claim to beat “industry” using a straw-man baseline. Name specific commercial controllers only when version, configuration, model boundary, inputs, hardware and results are documented and reproducible. A paper, vendor claim, simulation, and HIL result are different evidence classes.

## Proposed quantitative targets

These are goals for a future preregistered experiment—not current results. Freeze service floors, engineering tolerances, hardware, and primary outcomes before running comparisons.

| Dimension | Proposed target | Pass/fail interpretation |
|---|---|---|
| Scoring integrity | 100% of defined negative controls rejected; zero false-pass outcomes in the test set | Any known floor-breaching control reported as eligible fails the benchmark |
| Critical-service performance | At least **10% lower relative criticality-weighted unserved service-fraction-hours** than the strongest feasible baseline on held-out episodes | This time-integrated normalized deficit is not a physical unit sum; publish per-service physical shortfalls separately and require uncertainty to support improvement |
| Outage persistence | Lower longest consecutive system-level service-floor breach duration than the strongest eligible baseline on held-out compound events, without increasing breach counts | Preserve timeline and event boundaries; equal total breach-hours must not hide longer continuous loss |
| Endurance before failure | Longer time from event start to the first critical-service floor breach than the strongest eligible baseline on held-out outage trajectories | Treat traces without an observed breach as right-censored at the trace horizon; do not interpret them as infinite endurance |
| Critical-floor protection | No new candidate floor failure on an episode/service where the comparator passes, unless explicitly approved through prior safety review | New critical-floor breach blocks an unqualified “better” claim |
| Tail performance with credible probabilities | Aim for **15% lower CVaR at 95% confidence** for preregistered shortfall loss than the strongest feasible baseline | Report event-level outcomes, uncertainty, probability provenance, and sensitivity |
| Service-specific tail safety | Report expected loss, CVaR95 where probabilities are defensible, worst-case shortfall, and breach probability for every declared critical service; no aggregate improvement claim may hide a critical-service regression | Service-specific hard floors remain independently binding; inactive services report null losses rather than zeros |
| Probability-free stress performance | Lower worst-case shortfall and no increase in floor-breach scenarios relative to the strongest baseline | Same frozen stress set; make no frequency or expected-value claim |
| Fuel/resupply | Improve the fuel/resupply Pareto frontier at matched service outcomes; pre-approve any tolerated trade-off | Fuel or cost gains cannot excuse critical-service failure |
| Generalization | Improve on held-out compound hazards and report degradation by hazard family and geography | Keep Arctic land, Antarctic stations, and mobile/ice platforms distinct unless transfer is justified |
| Decision execution | 100% valid outputs inside the frozen decision deadline on named target hardware | Publish p50/p95/p99 latency, peak memory, platform energy, misses, and fallback behavior |
| Evidence reproducibility | 100% of runs link exact commit, input/config hashes, scenario IDs, units, model versions and result hashes | Missing lineage makes a run non-comparable and not independently reproducible |
| HIL progression | 100% of applicable, selected test cases pass at the agreed HIL boundary before a site trial | Project gate only—not a claim of IEC conformance |

Compare against the strongest *eligible* baseline, never a selectively weak one. Report absolute values beside percentages. The system-weighted tail score must be accompanied by per-service tails: in a synthetic example, total shortfall CVaR can be materially lower than water shortfall CVaR while the system average still looks acceptable. A 10% relative gain may be trivial at low absolute risk; owners must also set absolute service minima. Use episode/block-level uncertainty methods that preserve temporal correlation; do not treat each timestep as an independent sample. If too few independent sites or severe events exist, state that limitation rather than manufacture statistical power.

## Long-horizon resilience: benchmark the trajectory, not just the storm

A further paradigm shift is to evaluate a **continuous operating trajectory across seasons and repeated hazards**, not just one representative outage at a time. Recent research argues that event-by-event resilience can be hard to interpret for climate risks that evolve across decades; the benchmark should therefore keep event-specific tests, while also measuring system performance across long horizons and multiple plausible futures ([Nature Reviews Electrical Engineering, 2026](https://www.nature.com/articles/s44287-026-00322-6)). Related resilience-curve research emphasizes event-level outage volume, nadir, duration, and restoration dynamics that time-averaged statistics can conceal ([2026 review](https://pmc.ncbi.nlm.nih.gov/articles/PMC12905357/)).

Protocol requirements for a future trajectory-capable simulator:
- Carry state continuously across events: battery state of charge, thermal storage, fuel, water reserves, equipment wear, repair queues, spare inventory, and forecast/telemetry state. Never silently reset resources between storms.
- Separate seasonal weather sequences from independent stress-injection tests. Label whether the sequence is observed, calibrated, generated, or adversarially constructed.
- Score the whole horizon and each event window: critical-service shortfall, total and longest consecutive breach duration, breach-window counts, time to first breach, recovery, reserve exhaustion, lifecycle fuel/resupply, and maintenance backlog.
- Use multiple climate/resource futures and sensitivity bounds. Do not claim future occurrence probabilities unless their estimation is justified; use probability-free scenario envelopes where they are not.
- Preserve cold-start, black-start, and replenishment transitions between windows. A later storm should encounter whatever state the earlier event leaves behind.
- Freeze the scenario generator and seeds before final evaluation; separately report coverage of hazard families and compound dependencies rather than assuming a finite scenario list is exhaustive.

This protocol is a proposed next step, not a feature already implemented by this trace scorer. The current scorer preserves temporal ordering in provided traces, but it neither generates weather nor simulates equipment state or multi-year asset degradation.

## Frozen hazard and fault matrix

Start from `../STRESSOR_MEASUREMENT_MATRIX.md`. Use traceable cases and compound event sequences, not only isolated faults:

- Renewable scarcity during polar night, low wind, icing, and cold-related battery derating.
- Prime-power or inverter/control loss while critical demand remains high.
- Delayed fuel/spares resupply coincident with a prolonged outage.
- Communications loss, stale telemetry, sensor drift, and common-cause failures.
- Black-start/cold-restart, frozen water/wastewater systems, and snow/ice access blockage.
- Maintenance delays, limited spares, and technician constraints in cold/gloved conditions.

For a non-applicable stressor, record **not applicable and why**; never silently drop it. Keep calibration data, tuning data, and final hold-out episodes separate. Preserve event duration and state carry-over, and test both single faults and correlated/compound events.

## Evidence ladder: what a result is allowed to claim

| Level | Evidence | Permitted conclusion |
|---|---|---|
| E0 — contract | Unit tests, schema checks, negative controls, metamorphic tests | Scorer/report contract behaves as tested |
| E1 — synthetic traces | Hand-authored or generated fixtures with stated assumptions | Metric pipeline works on those synthetic inputs |
| E2 — calibrated model | Traceable inputs, validated component/system models, held-out episodes and sensitivity | Model-conditional comparison within validated scope |
| E3 — HIL/lab | Documented testbed, real controller hardware or power hardware as appropriate, repeatable fault injection | Behavior validated at stated lab/HIL boundary |
| E4 — field/commissioning | Qualified review, site-representative tests, operating constraints and monitored outcomes | Evidence limited to tested configuration, site, and period |

Evidence does not automatically rise because a workflow passes. Synthetic-only output is not physical validation, controller certification, engineering safety evidence, or superiority to state of the art.

## Industry anchors and limitations

- [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175) covers HIL, commissioning, and periodic testing for microgrid monitoring, control, and energy-management systems. Use it to build a gap map; this benchmark alone does not show compliance.
- [NREL/NLR, Hardware-in-the-Loop Test Bed and Test Methodology for Microgrid Controller Evaluation](https://www.nrel.gov/docs/fy20osti/71338.pdf) provides controller-evaluation methodology.
- [Reliability as a design principle: A systematic review and integrated framework for renewable-based microgrids (2026)](https://www.sciencedirect.com/science/article/pii/S2590174526003612) motivates time-coupled adequacy, correlated scarcity, and service-critical tail measures.
- [Quantifying Microgrid Resilience by Determining Survivable Durations of Random Outages (IEEE Access, 2025)](https://doi.org/10.1109/ACCESS.2025.3626579) motivates reporting how long critical service can be sustained before failure, rather than only average service fractions.
- [IEEE 2030.8: Testing of Microgrid Controllers](https://standards.ieee.org/ieee/2030.8/6169/) is another relevant test reference.

The current code demonstrates metric calculation on synthetic fixtures only. The next scientific milestone is not a higher dashboard score; it is a frozen, reproducible, site-relevant comparison whose evidence can falsify the candidate's value proposition.
