# Energy design search: robustness and fair-comparison protocol v0.1

**Status:** benchmark protocol; five-case synthetic panel and actual-dispatch CI step are implemented, but the latest end-to-end run has not yet completed successfully.  
**Applies to:** Symthaea candidate generation, Symtropy contracts/scenarios, Sol-Atlas compilation/dispatch, and the independent Symtropy auditor.  
**Claim ceiling:** finite-model benchmark until calibrated site inputs, validated component models, and independent reference comparisons exist.

## Why this is the next paradigm shift

A deterministic sizing sweep answers “does this pipeline enumerate and audit a declared small model correctly?” It does not answer “will this design serve people through a bad week, and is the additional resilience worth the cost?” Do not collapse those into one score.

The new target is **evidence-aware, multi-scenario design search**: preserve a common candidate architecture across scenarios, reveal operational recourse per scenario, keep unknowns explicit, and compare algorithms under equal evaluation budgets. Make robust service performance the first-class result; economics and lifecycle factors count only when explicitly supplied with units, provenance, and applicability.

This extends—not replaces—the frozen two-interval fixtures, independent oracle, negative controls, and exact byte receipts.

## Benchmark ladder

| Level | Question | Required evidence | Pass condition |
|---|---|---|---|
| L0: contract | Is each candidate structurally well-formed? | Schema, typed ports/units, claim ceiling | All invalid-contract controls rejected |
| L1: finite-model correctness | Does dispatch obey declared balance, limits, SoC and outage rules? | Frozen known answers + independent bounded enumerator | Exact frontier/count agreement; UNKNOWN on cap |
| L2: scenario robustness | Does one fixed design survive a deterministic stress envelope? | Pinned scenario IDs, per-scenario requests and receipts | Report every scenario; no silent drop |
| L3: search quality | Which search policy finds better designs for equal effort? | Matched candidate/dispatch budgets and held-out scenarios | Reproducible per-objective comparison |
| L4: calibrated site case | Does it outperform existing energy tools on a matched task? | Meter/load, tariff, resource, equipment, outage and validation evidence | Independent matched-model comparison, with uncertainty |
| L5: field evidence | Does the installed system behave as predicted? | Instrumented commissioning and operational records | Pre-registered prediction error and safety criteria |

Do not claim L3 or higher from L0/L1 success.

**Current L3 precursor implemented:** `energy_search_policy_replay` replays the already-qualified 39-candidate matrix in proposal orders at fixed unique-proposal budgets (1, 3, 6, 12, 24, 39). It compares the reference templates plus Symthaea manifest order, lexical order, a transparent battery/PV structural heuristic, and 30 fixed seeded random orders (33 runs total). At every budget it reports p10/p50/p90 and min/max over 30 seeds for discovered envelope-frontier vector count, worst-case essential-unserved quanta and essential-service/cost gaps. The full-domain envelope frontier is the exact finite-domain reference; outputs include frontier-vector recall and coordinate-wise gaps, with no weights or invented probabilities. Because every candidate was already evaluated to qualify the matrix, this is a cached-outcome discovery-order analysis—not a measurement of live policy runtime, true adaptive search, or industry superiority. It is an initial reproducible budget-quality curve, not the completed L3 claim.

## Scenario registry v0.1

The first panel should be **deterministic and non-probabilistic**. Each row is a named transform over a pinned parent candidate and a pinned baseline input bundle. Store parent digest, transformed child digest, mutation path(s), before/after values, units, model version and claim ceiling. Refuse unlisted mutations.

1. **nominal**: current known-answer baseline, byte-for-byte frozen.
2. **pv_derate_25, pv_derate_50, pv_derate_75**: reduce PV availability independently of nameplate rating.
3. **pv_window_delayed**: shift availability later while preserving the total available energy where the model can represent this distinction.
4. **pv_window_shortened**: preserve peak/rated power but reduce available-energy duration.
5. **outage_early, outage_late, outage_long**: change outage start and duration; imported grid energy must be zero during outage intervals.
6. **demand_surge_110, demand_surge_125, demand_surge_150**: scale required service, not the generation claims.
7. **reserve_25, reserve_50**: raise terminal/operational reserve requirements explicitly.
8. **combined_pv50_outage_long_surge125**: a composed held-out stress test; record each mutation and its composition order.
9. **evidence_unknown_grid_factors**: cost/carbon factors absent; expected outcome is *unrankable for those objectives*, never silently scored as zero.
10. **mutation_attack variants**: tampered receipts, objective vectors, stale parent hash, hidden extra mutations, altered outage limits, forged reserve, omitted scenario rows.

The currently frozen five-case panel is three hourly intervals at 1-kWh energy quantum. It can test total PV loss, outage shift by one interval, doubled essential demand and a one-quantum terminal reserve, but cannot support meaningful 25/50/75% PV derates or a rich solar-window/outage-duration sweep. Those cases need a finer declared energy quantum and more intervals, plus new frozen known answers and an independent reference check. Do not describe the current panel as a delayed/shortened solar-window or long-outage benchmark.

If scenario probabilities are not evidence-backed, publish deterministic envelopes only: worst-case essential unserved energy, worst-case reserve shortfall, longest continuous deficit, maximum import/cost/carbon among explicitly known values, and per-scenario feasibility. Do not report expected values or CVaR based on invented probabilities.

## First frozen panel implemented (not yet CI-qualified)

`fixtures/energy-scenario-panel-v0.1.json` now contains five deterministic synthetic cases. Each scenario declares its parent and exact JSON-pointer mutation list, embeds its request, and freezes the full feasible-schedule objective multiset plus expected Pareto frontier. The independent Rust reference test enumerates each case and derives the frontier from the complete objective multiset. The E2E workflow is configured to execute the actual Sol-Atlas CLI, audit every output and emit a digest-bound panel report.

Frozen finite-model expectations:

| Scenario | Feasible complete schedules | Pareto vectors |
|---|---:|---:|
| Nominal | 9 | 4 |
| Total PV loss | 3 | 3 |
| Outage shifted to final interval | 9 | 4 |
| Essential demand doubled in outage/recovery | 10 | 4 |
| Terminal reserve raised to one quantum | 4 | 2 |

The latest end-to-end run was still queued when this protocol was last updated; these values are frozen answers and static reference calculations, **not** a successful actual-Sol-Atlas run or site-resilience evidence. The next step is the full 39 candidate × 5 scenario matrix (195 candidate/scenario pairs) with explicit failed/unknown rows.

## Candidate-level robustness matrix

The new candidate-scenario profile defines five transforms for all three reference topologies and 36 sizing candidates: nominal, total PV loss, early outage, doubled essential demand, and a 1 kWh terminal reserve where a battery exists. The transform tool records exact parent/child hashes and each declared mutation. Designs without batteries are marked not applicable for the reserve scenario.

The end-to-end workflow now runs schema validation, Sol-Atlas compilation, compilation-receipt verification, dispatch, and independent audit for every pair. The expected matrix is 39 candidates × 5 scenarios = 195 rows. The gate preserves failed or unknown rows, checks exact coverage and digest bindings, and fails the aggregate result if any pair is absent or unqualified. Matrix report v0.2 serializes each scenario's full solver frontier and independently recovered frontier as objective vectors; the v0.6 benchmark ledger deserializes and rechecks exact vector equality and frontier counts. This makes future regret/dominance calculations auditable from the ledger rather than inferring trade-offs from frontier sizes. A Pareto frontier count is not a quality score: preserve full vectors, compare objective-by-objective, and do not combine coordinate-wise best values into a claim that one schedule achieves them jointly. The standalone `energy_candidate_robustness_report` now derives digest-bound per-candidate reports from a qualified v0.2 matrix. It preserves each full scenario frontier; computes axis-wise best values and a worst-case-of-per-scenario-best envelope; and labels the envelope as coordinate-wise, not a jointly attainable schedule. It reports strict envelope dominance, equal envelopes, and incomparability without scalar weights. The v0.6 ledger independently recomputes every per-scenario axis-best value and worst-case envelope and verifies all source frontier vectors and reciprocal relation sets. The derived report is an additional evidence artifact, not a substitute for raw scenario data. This remains a two-interval synthetic robustness test, not a site-real or long-duration-outage result.

## Fair search comparison

Use one shared candidate contract, scenario panel, exact evaluator, auditor, objective definitions, and provenance rules for each arm:

- **Reference templates:** grid only; PV + grid; PV + battery + grid.
- **Exhaustive sizing:** for tiny domains, enumerate every declared tuple and use as an oracle.
- **Seeded random:** sample only from the exact same allowed design space; publish algorithm/version and seed.
- **Constraint-guided search:** prune only with declared sound constraints; record each rejection reason and prove that no rejected design can pass the same constraints.
- **Symthaea-assisted search:** use its generated proposals/prioritization, but subject every candidate to the same schema, compiler, exact dispatcher and independent audit. Measure contribution rather than assume it.
- **External reference:** only when the same boundary, data, time horizon, technology constraints and objective semantics can be represented.

Budget tiers: 36, 128, 512, and 10,000 **unique proposals**, with duplicates counted and reported rather than refunded. Separately report exact dispatch evaluations, rejected proposals, reference expansions, wall-clock time and peak memory. Match hardware/runner, toolchain, cached/warm state, and objective/scenario bundle. Include one-time generation/build overhead in an end-to-end result; report amortized repeated-run timing separately. Never compare a warmed fast path against a cold competitor run.

Use disjoint scenario families for **design/tuning**, **validation**, and **held-out final evaluation**. A hidden or held-out test family must not guide the proposal policy or its parameters. Pin all random seeds and commit every generated input digest.

## Metrics: do not hide the trade-off

Report each candidate and method with:

- Feasible fraction; false-feasible count; and independent negative-control detection rate.
- Essential and flexible unserved energy by scenario, worst-case shortfall, and longest continuous deficit.
- Reserve shortfall, terminal SoC, grid imports during allowed intervals, curtailment, and energy-balance residual.
- Grid operating cost and emissions only when explicit input factors exist; label every factor by units, source, date, boundary and evidence class.
- Pareto-frontier size, exact-frontier recall on tiny oracle cases, and hypervolume only with objectives normalized against pre-registered fixed bounds.
- Unique candidates generated, duplicate rate, exact dispatch evaluations, reference expansions, wall time, memory, reject reasons, and amortized/non-amortized generation cost.
- Regret versus the exact oracle on enumerable instances; on larger instances label the comparator “best known,” not “optimal.”

Do not select a single weighted score after seeing the results. If stakeholders need a scalar decision later, pre-register weights and publish the unweighted vector alongside it. For distributional service claims, report subgroup or scenario-specific results rather than letting average performance obscure a critical-load failure.

## Pre-registered hypotheses and gates

These are **targets to test, not achieved results**:

1. 100% detection of the current and expanded mutation controls; zero false-feasible outcomes in the audited panel.
2. 100% exact frontier/count recovery for small cases when the independent oracle completes. Any budget/depth overflow is UNKNOWN.
3. 100% reporting coverage across registered scenarios and candidates; no missing case silently omitted from averages.
4. At equal unique-proposal and exact-dispatch budgets, test whether constraint-guided and Symthaea-assisted search improve held-out Pareto quality over seeded random and reference templates. An initial practical-effect hypothesis is ≥10% higher normalized held-out hypervolume **without worsening worst-case essential unserved energy or increasing false-feasible results**; preregister normalization bounds and report confidence intervals before judging it.
5. Publish the full budget-quality curve, not just the best run. Repeat stochastic policies across multiple fixed seeds and report spread.
6. A claim of “better than industry” requires a calibrated matched site case and rerunnable inputs. If model boundaries do not match, report a qualitative scope difference—not a head-to-head score.

## Industry comparison: correct tool for each question

- [NLR REopt](https://www.nlr.gov/reopt/curriculum/what-is-reopt) is the closest facility-scale comparator for choosing on-site generation/storage sizes, electricity-cost optimization, and outage resilience. Compare using matching utility rates, load, technologies, analysis horizon and outage assumptions.
- [NREL SAM battery storage](https://sam.nrel.gov/index.php?Itemid=360&id=94&option=com_content&view=article) is an important performance/dispatch and storage-degradation reference, not a substitute for every capacity-planning model. Use it to stress-test battery aging and dispatch where model inputs align.
- [PyPSA stochastic optimization](https://docs.pypsa.org/latest/user-guide/optimization/stochastic/) supports multi-scenario investment/dispatch and CVaR-based risk preference. It is a natural reference for scenario-aware planning once Energy Studio exposes compatible multi-interval inputs.
- [GenX](https://genxproject.github.io/GenX.jl/stable/Model_Concept_Overview/model_introduction/) addresses constrained power-system capacity expansion with operational dispatch, storage, reserves, transmission and policy dimensions. It is relevant only for planning cases with matching scope and time/spatial resolution.

These tools solve overlapping but non-identical problems. A small Rust integer scheduler should not claim to beat them based on runtime on a two-interval toy case.

## Implementation order

1. Fix all observed compile/test blockers before using outputs; keep tests and end-to-end runs separate.
2. Freeze the current v0.1 candidate/request/report golden answers unchanged.
3. Keep the frozen five-case panel and source-bound mutation receipts; complete the pending CI qualification.
4. Run the 39 × 5 candidate/scenario matrix through actual Sol-Atlas dispatch and the independent audit; require exactly 195 accounted-for rows.
5. Add sound constraint-pruning, seeded-random, and Symthaea-assisted policy arms using the same evaluator, starting with the 36-tuple oracle space.
6. Expand the design domain before measuring 128/512/10,000 unique-proposal tiers; separate tuning, validation and held-out scenario families.
7. Publish the digest-bound ledger with budgets, per-objective results, failures, and scope limitations.
8. Only then add calibrated site evidence and matched REopt/SAM/PyPSA/GenX comparisons.

## References

- NLR, “What Is REopt?”: https://www.nlr.gov/reopt/curriculum/what-is-reopt
- NREL SAM, “Battery Storage”: https://sam.nrel.gov/index.php?Itemid=360&id=94&option=com_content&view=article
- PyPSA, “Stochastic Optimization”: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- GenX, “Model Introduction”: https://genxproject.github.io/GenX.jl/stable/Model_Concept_Overview/model_introduction/
- U.S. DOE, “An Integrated and Iterative Multiscale Modeling Framework for Robust Capacity Expansion Planning”: https://eesm.science.energy.gov/publications/integrated-and-iterative-multiscale-modeling-framework-robust-capacity-expansion
