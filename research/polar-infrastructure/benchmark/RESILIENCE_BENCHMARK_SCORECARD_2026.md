# Resilience benchmark scorecard and industry-comparison protocol (2026)

**Status:** proposed / not executed. This is a preregistration template, not an empirical result.
**Current evidence ceiling:** environmental catalog and synthetic contract tests exist; the Gobabeb station file has not been acquired or ingested, the POWER comparison has not been scored, and no policy has demonstrated superiority over an industry baseline.

## 1. The paradigm shift: score evidence and service, not scenario count

A large scenario catalogue is useful for planning but is not itself proof of resilience. The benchmark should become a reproducible challenge in which each result is tied to an admitted source trace, explicit model boundary, fixed information budget, paired disturbance, and independently auditable outcome.

The benchmark therefore has three linked but separate tracks:

1. **Input validity:** are source bytes authentic, temporal semantics correct, missingness preserved, transformations reproducible, and the input appropriate for the declared use?
2. **Model validity:** is the relevant physical model verified against known-answer cases and validated against independent measurements or accepted reference tools?
3. **Operational resilience:** under the same disturbance and information budget, which controller/configuration preserves critical service, respects hard constraints, recovers fastest, and does so at an acceptable cost?

Success at one track does not prove success at the next. A SHA-256 hash establishes byte identity, not sensor correctness. Agreement with a weather reanalysis does not validate an electrical or thermal solver. A good average score does not establish safety under tail events.

## 2. Industry comparison must be layer-matched

Compare systems at the level of the actual claim; do not make a single overall feature comparison across unlike tools.

| Layer | Relevant comparator | What it can test | What must not be claimed |
|---|---|---|---|
| Microgrid planning and dispatch | NLR's REopt | Cost-aware sizing/dispatch and modeled outage survivability under a shared site, load, tariff and technology definition | That REopt is an electrical transient simulator or that a different input/model boundary is a fair head-to-head |
| Hybrid system design and sensitivity | HOMER Pro / HOMER Front | Feasible-system simulation, optimization, economics and sensitivity under matched component assumptions | That the product's optimization output automatically validates site physics or real-world resilience |
| Distribution-network physics | OpenDSS | Distribution-system power-flow/network cases, using an explicitly matched feeder and equipment model | That policy or planning metrics from a different network model are numerically comparable without an adapter and a common reference |
| Environmental input quality | Gobabeb BSRN observations and NASA POWER | Site-specific observation-vs-reanalysis error, coverage, time alignment and event-window behaviour | That a weather-data comparison proves a controller is better |
| Symtropy system-level research claim | Same-case policy bake-off over admitted traces and an independently checked plant model | Whether an added policy/multiphysics coupling improves resilience within the same constraints and runtime budget | Any SOTA claim until the exact artifact, strong baselines, locked holdout and uncertainty analysis are published |

Relevant source documentation:
- NLR REopt: https://www.nlr.gov/reopt/about/index and https://developer.nlr.gov/docs/energy-optimization/reopt/
- HOMER's simulation / optimization / sensitivity workflow: https://support.homerenergy.com/docs/getting-started/solving-problems-with-homer-software
- EPRI OpenDSS: https://opendss.epri.com/
- GMLC resilience metrics report (DOE): https://gmlc.doe.gov/sites/default/files/resources/GMLC1.1_Vol3_Resilience.pdf
- IEEE PES report, *Power System Resilience Metrics and Evaluation Methods* (published 2026-06-08): https://resourcecenter.ieee-pes.org/publications/technical-reports/pes_tr_143_amps_060826
- BSRN/PANGAEA Gobabeb series: https://doi.org/10.1594/PANGAEA.968639
- NASA POWER validation service: https://power.larc.nasa.gov/docs/services/api/validate/

The official sources describe different but complementary capabilities. The proposed contribution is not to claim that Symtropy replaces each specialist tool immediately; it is to create a common, evidence-gated, scenario-paired evaluation harness around them where adapters and comparable models are available.

## 2.1 New benchmark axis: hierarchy, autonomy and interdependence

Recent industry guidance makes clear that resilience metrics need to be selected for the system level and operational context. IEEE PES published *Reliability and Resilience for New Grid Needs (TR148)* on 1 October 2026, addressing modern grids, inverter-based resources, extreme events and cyber/physical threats. A 2026 review also highlights critical function, time and decision-making hierarchy as useful dimensions for resilience measurement. These reinforce a benchmark gap: a system can appear resilient locally while a dependency or coordination failure causes service loss at a larger level.

Source: https://resourcecenter.ieee-pes.org/publications/technical-reports/pes_tr_tr148_itslc_102026
Review: https://doi.org/10.1016/j.epsr.2026.112736

Every scenario should declare **which hierarchy levels are represented** rather than collapsing them into one microgrid score:
- **Device/control:** inverter, battery, sensor, actuator or protective controller.
- **Site/microgrid:** power, thermal, water, communications and load-serving systems.
- **Community/multi-site:** inter-site resource sharing, logistics, shared spares and dependencies.
- **Operator/coordination:** centralized coordination, local autonomy, communication and restoration authority.

For each represented layer, declare its critical services, upstream dependencies, available actions, observation/forecast access, safety floors and time horizon. Record an explicit dependency-graph digest. Unmodeled layers must be marked not covered; no cross-layer guarantee is inferred from one layer's passing score.

### A new falsifiable experiment: resilience under coordination partition

On a validated, small multi-site test model, inject a communication partition or stale coordination feed while keeping the physical hazard sequence identical. Compare:
1. the current/static policy;
2. local autonomous fallback policy;
3. centralized coordinated policy under healthy communications;
4. the same centralized policy under communication delay/partition, if its behavior is defined;
5. a candidate hybrid policy that uses local fallback and reconciles state on reconnection.

Pair all arms by initial state, disturbance seed, service-contract digest and exogenous trace. Freeze what each arm knows at each time; do not leak future events to the fallback. Test both partition onset and recovery/re-synchronization.

Required outcome slices: critical service served by hierarchy; outage duration per layer; unserved energy/water/heat as applicable; hard-constraint violations; cascading failure count; time to detect loss of coordination; safe fallback latency; time to restore a consistent shared state; resource-sharing regret; communication volume; compute and runtime. Report worst layer and worst hazard group, not just network-wide averages.

**Research hypothesis, not a result:** local fallback plus explicit state reconciliation may reduce service loss under coordination outages relative to a centralized-only policy, while healthy-connectivity centralized optimization may retain advantages in shared-resource efficiency. The benchmark should quantify that Pareto trade-off rather than assume one architecture dominates.

## 3. Baseline ladder

Every deployable policy comparison should include the strongest available non-oracle baseline, not only a weak heuristic.

1. **No adaptation / static policy:** fixed reserve, fixed priority shedding or the current operational default. Document whether it is a meaningful incumbent.
2. **Transparent rule policy:** threshold triggers and priority dispatch with all thresholds frozen on development/validation data.
3. **Greedy or short-horizon policy:** myopic action selection under the same observation stream and action limits.
4. **Deterministic model-predictive control (MPC):** same forecast, horizon, measurements, actuator limits and compute budget as the candidate.
5. **Robust or stochastic MPC / uncertainty-aware optimization:** include when the use case and implementation support it; do not silently omit an available stronger baseline.
6. **External planning-tool reference:** REopt and/or HOMER where a compatible planning problem exists; OpenDSS for the relevant electrical network analysis. Pin the release/configuration and state whether a tool is used as an optimizer, simulator, or both.
7. **Candidate Symtropy policy:** all additional sensors, forecasts, memory, compute and model fidelity must be declared. Match the baseline information and resource budget, or report the advantage as a separate resource premium.
8. **Perfect-information oracle:** optional diagnostic upper bound using future disturbances. It is not a deployable competitor and must never be described as a fair real-time baseline.

The exact baseline set must be frozen before the locked test. Include a baseline whenever it is technically compatible and available; otherwise document why it is excluded, with no post-result exclusion.

## 4. Required outcome metrics

Do not collapse the entire result into one weighted score. Report an interpretable set of outcomes and a Pareto frontier.

### Hard gates and integrity
- Invalid/unverified traces admitted to scoring: **0**.
- Any result without exact input, configuration, model, source and code digests: **ineligible**.
- Known-answer negative controls that are falsely accepted: **0**.
- Hard engineering constraint violations: report the count and severity. A zero count in a finite test suite is a pass for those tested cases, not a universal safety guarantee.
- Unresolved numerical/model validity failures: disclose and block the related claims; do not turn an unevaluated case into a pass.

### Critical-service outcomes
Use the measures appropriate to the modeled service, at minimum:
- Critical energy demand not served (kWh) and critical-load fraction served (%).
- Critical-service outage hours / customer-hours, where customer or service aggregation is modeled.
- Time to restore each required service and time to full recovery.
- Number, duration and severity of hard-constraint excursions.
- Equipment degradation/damage and recovery cost, only when represented by a validated model.
- Fuel/grid imports, lifecycle cost and emissions where the boundary supports them.

### Tail risk, robustness and generalization
- 95th percentile and CVaR at 95% for unserved critical energy and recovery time; define the CVaR convention.
- Worst-group outcomes by hazard family, compound scenario and operating regime.
- Performance drop from development to locked test, plus out-of-domain cases identified before evaluation.
- Regret to the perfect-information oracle, reported only as a diagnostic.
- Calibration of predicted failure probabilities (Brier score and reliability plot) only if the policy makes probabilistic claims.

### Input quality and resource use
- Station-vs-reanalysis matched-window coverage, bias, MAE, RMSE, p95 absolute error, slope/intercept and event-conditioned error; report units and time aggregation.
- Missing-window rate, longest gap, duplicate timestamps, alignment errors, and raw-versus-screened metrics where sensor QA requires it.
- Wall time, CPU/GPU, peak memory and energy where measurable; record hardware, software versions and concurrent-load assumptions.
- Number of simulator calls, optimization evaluations, function evaluations, and oracle queries. Same budget means the same declared unit of computational or information access, not merely the same number of policy steps.

The DOE GMLC metric catalogue emphasizes service interruption, energy not served, recovery time and costs; the 2026 IEEE PES report also frames resilience metrics as context-dependent. Report multiple operationally meaningful measures rather than relying on a universal scalar.

## 5. Fairness and anti-leakage protocol

- **Paired disturbance:** every policy receives the same trace, starting state, failure/injection point and scenario seed.
- **Equal information:** freeze forecast source, look-ahead horizon, sensor visibility and action constraints. If one policy uses richer information, publish a separate ablation.
- **Temporal holdout:** split whole events/months/seasons/site-years. Never random-shuffle adjacent time samples into train and test.
- **Locked test:** keep model/policy selection and threshold tuning outside the locked test. If the test is used to debug or tune, it becomes development data and needs a new holdout.
- **Uncertainty:** compute paired confidence intervals at the episode/day/year level, not by treating highly autocorrelated minutes as independent observations. Publish block definition, seed and estimator.
- **Multiple comparisons:** declare the primary endpoint before running; distinguish confirmatory tests from exploratory slices and adjust or clearly label multiple policy comparisons.
- **Negative controls:** include known failure, bad unit, time-zone mismatch, missing-source, corrupted digest, insufficient-coverage and invalid-sensor cases. These must be rejected or explicitly excluded from scoring.
- **Independent replay:** a second run in a clean environment should reproduce normalized artifact hashes and the scorecard within a declared numerical tolerance. If a simulator is nondeterministic, report its stochastic design and uncertainty instead of fabricating byte-identical numerical output.

These choices follow current climate-prediction preprocessing recommendations: temporal dependence and preprocessing can leak information across splits, so block design and disclosure are part of the scientific result. See https://journals.ametsoc.org/view/journals/bams/107/6/BAMS-D-24-0292.1.xml.

## 6. Proposed target for a first defensible superiority claim

This is an **aspirational pre-registration target**, not a measured result:

1. Pass every input/model admission gate relevant to the case; publish the exact artifacts and exclusions.
2. Show at least a **10% relative reduction in locked-test CVaR95 critical energy not served** versus the strongest admissible deployable baseline, with a paired 95% confidence interval excluding zero improvement.
3. Demonstrate non-inferiority on hard safety constraints and on a pre-declared economic or resource budget. Set the allowed cost/runtime margin from project requirements before the test; do not select it after looking at results.
4. Show no material regression in any pre-declared critical hazard group. If there is a trade-off, report the Pareto frontier and the affected groups rather than hiding them in an aggregate.
5. Repeat across a second independent time block or site before making a cross-site/generalization claim.

If the sample is too small or the interval crosses zero, report the estimate and uncertainty honestly; do not weaken the endpoint, move the holdout or cherry-pick a policy. The 10% target should be changed only before locking the evaluation, with the rationale recorded.

## 7. Evidence maturity ladder

| Level | Admission criterion | Allowed statement |
|---|---|---|
| E0 — synthetic contract | Known-answer structural and negative-control tests | The evaluator/contract behaves as specified on the synthetic cases |
| E1 — source integrity | Acquired raw trace, license/citation, provenance, hashes and repeatable normalization | This exact trace can be acquired and transformed reproducibly |
| E2 — data validation | Independently reviewed time/unit/QC semantics and held-out station/model comparison | The named variables are characterized for the stated site, time and use |
| E3 — model verification | Physics/solver checks, conservation/invariants, numerical convergence and reference cases | The declared model is verified within tested bounds |
| E4 — operational bake-off | Paired policies, strong baselines, fixed budgets, locked test and uncertainty | The candidate performed as reported on the named test suite |
| E5 — external replication | Independent replay or independent site/episode reproduces the claim | Evidence supports generalization within the stated deployment envelope |

A later level does not retroactively erase earlier caveats. Each report must state the exact level reached per claim.

## 8. Immediate experimental sequence

1. Acquire one complete PANGAEA month as raw bytes, review the actual metaheader and station data-release terms, and commit neither restricted data nor unreviewed measurements.
2. Build a fixture from the exact tab-delimited file header and metadata; write a source-specific parser that preserves raw values, quality flags and missing markers.
3. Align one variable (first choice: SWD) with a pinned NASA POWER request on a clearly defined daily or hourly interval. Record paired coverage before calculating error.
4. Run simple sanity and alignment negative controls, then the calibration/validation/locked split already specified in the Gobabeb validation plan.
5. Only after the environment input is admitted, use it in a domain model whose plant assumptions and hard constraints are independently tested.
6. Run the baseline ladder under paired disturbances and report per-group tail risk, cost, hard-constraint outcomes and uncertainty.
7. Add a separate hierarchical coordination-partition experiment only after the dependency graph, communication semantics, local fallback actions and reconnection behavior are specified and verified.

At the current PR head, these are planned experiments. No real Gobabeb raw file, station-vs-POWER score, controller bake-off or SOTA advantage is claimed.
