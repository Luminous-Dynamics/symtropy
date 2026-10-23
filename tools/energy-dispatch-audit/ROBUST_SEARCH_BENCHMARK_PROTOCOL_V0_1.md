# Energy design search: robustness and fair-comparison protocol v0.1

**Status:** protocol proposal; not yet an executed multi-scenario benchmark.  
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

The current two-interval contract cannot express a convincing delayed renewable window, long outage chronology, or realistic reserve policy. Those rows must remain marked **not executable on v0.1** until the time-series/scenario contract supports them. Do not simulate a multi-interval feature by relabeling two intervals. First implement a versioned scenario envelope and typed transform receipt, then freeze a matching model and golden case.

If scenario probabilities are not evidence-backed, publish deterministic envelopes only: worst-case essential unserved energy, worst-case reserve shortfall, longest continuous deficit, maximum import/cost/carbon among explicitly known values, and per-scenario feasibility. Do not report expected values or CVaR based on invented probabilities.

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
3. Specify and schema-validate a typed multi-interval scenario bundle and mutation receipt.
4. Add 1–2 hand-computable multi-interval known-answer scenarios, with independently derived frontier and malicious-receipt controls.
5. Add a deterministic scenario-panel runner and coverage gate; archive per-scenario raw artifacts.
6. Add matched-budget baseline runners (templates, exhaustive tiny oracle, seeded random, constraint-guided, Symthaea-assisted).
7. Run held-out robustness panels; publish a digest-bound ledger with budgets, per-objective results, uncertainties, failure cases and scope limitations.
8. Only then add real-site evidence and external REopt/SAM/PyPSA/GenX comparisons.

## References

- NLR, “What Is REopt?”: https://www.nlr.gov/reopt/curriculum/what-is-reopt
- NREL SAM, “Battery Storage”: https://sam.nrel.gov/index.php?Itemid=360&id=94&option=com_content&view=article
- PyPSA, “Stochastic Optimization”: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- GenX, “Model Introduction”: https://genxproject.github.io/GenX.jl/stable/Model_Concept_Overview/model_introduction/
- U.S. DOE, “An Integrated and Iterative Multiscale Modeling Framework for Robust Capacity Expansion Planning”: https://eesm.science.energy.gov/publications/integrated-and-iterative-multiscale-modeling-framework-robust-capacity-expansion
