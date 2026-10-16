# Destination Portfolio Comparator: Benchmark Targets v0.1

**Status:** proposed engineering acceptance criteria and pilot-measurement protocol. No target below is claimed to have been achieved. Numeric fixture data is synthetic and must not be reused as a site threshold.

## Why these benchmarks

The UN Statistical Framework for Measuring the Sustainability of Tourism integrates economic, environmental and social dimensions; it is an organizing and measurement framework, not one universal score. OECD's 2026 tourism policy analysis calls attention to local value retention, resident sentiment, visitor flows, public-service pressure, and measurement gaps in community and housing impacts. These support a multi-dimensional evidence model rather than optimizing only attendance or gross revenue:

- [UN Tourism, Statistical Framework for Measuring the Sustainability of Tourism](https://www.untourism.int/tourism-statistics/statistical-framework-for-the-sustainability-of-tourism)
- [OECD, Tourism Trends and Policies 2026: Enhancing the social benefits of tourism](https://www.oecd.org/en/publications/oecd-tourism-trends-and-policies-2026_3fd3cd75-en/full-report/enhancing-the-social-benefits-of-tourism_43631a73.html)
- [OECD Tourism Trends and Policies 2026 overview](https://www.oecd.org/en/publications/2026/06/oecd-tourism-trends-and-policies-2026_13326fe2.html)
- [NIST AI RMF Measure guidance](https://airc.nist.gov/airmf-resources/playbook/measure/), useful for test-set documentation, uncertainty and independent evaluation patterns.

## Additional research: adversarial and interaction coverage

The target suite should combine a small set of explicit negative controls with bounded exhaustive oracle checks and higher-order interaction cases. NIST's combinatorial testing work shows that t-way interaction coverage can substantially reduce test-set size while exposing interacting configuration faults; it is a test-design technique, not a guarantee that all faults are found. Use full pairwise coverage for the declared factor model, then selected 3-way combinations for interactions that can change constraint status or scenario feasibility.

Sources: [NIST Combinatorial Testing](https://www.nist.gov/programs-projects/combinatorial-testing) · [NIST SP 800-142, Practical Combinatorial Testing](https://csrc.nist.gov/pubs/sp/800/142/final) · [Rust proptest documentation](https://docs.rs/crate/proptest/latest). The current tests use dependency-free exhaustive loops; a future property-testing dependency is optional, not required to claim these exhaustive cases.

## New paradigm: measure what can change the decision

Optimizing model detail is not enough. The next experiment should target uncertainty that could change which options are considered non-dominated. The comparator now reports:
- conservative and optimistic-envelope Pareto frontiers;
- candidates whose frontier membership changes between those endpoint views;
- one-objective-at-a-time endpoint leverage (which declared measurement dimension moves frontier membership when shifted from conservative to optimistic);
- per-objective unknown-observation counts, candidate IDs and evidence references, coupled with known-interval leverage as an evidence-gap triage view;
- stable membership and a clear caveat that different optimistic endpoints may come from different scenarios.

This is a **measurement-priority heuristic**, not formal value of information. A formal expected value of information calculation would additionally require defensible joint probability distributions, a locally approved utility/decision rule, measurement cost, and a defined decision horizon. We must not invent these from the synthetic fixture.

### New benchmark gates

| Gate | Benchmark | Proposed acceptance target | Method |
|---|---|---|---|
| B9: endpoint sensitivity | Frontier membership is computed separately from conservative and optimistic envelopes; one-objective-at-a-time leverage results are reproducible | 100% agreement with an independent reference on targeted tests; hard-infeasible candidates remain excluded in every frontier | Synthetic test where interval endpoints provably flip frontier membership plus negative controls |
| B10: measurement prioritization integrity | Report heuristic status and warn that optimistic objective endpoints may not be jointly attainable | No output labels the heuristic as expected VOI, causal impact or probability | Schema assertions and report review; formal VOI is a separate future feature |
| B11: unknown-evidence triage | Unknown observations are surfaced by objective and candidate, with evidence references | 100% of unknown outcomes appear in the correct objective gap list; no numeric imputation | Mutate known outcomes to unknown and compare counts/IDs/references against an independent audit |

## Benchmark ladder

| Gate | Benchmark | Acceptance target | Measurement method |
|---|---|---|---|
| B0: schema integrity | Every input objective has a declared unit/direction/non-negative per-objective dominance tolerance; every candidate appears in every paired scenario; every required constraint has an explicit state and evidence reference | 100% of malformed/incomplete mutation fixtures rejected; zero silent defaults | Rust unit tests with one mutation per failure mode; preserve failing fixtures |
| B1: safety precedence | One hard failure in any scenario excludes the candidate; unknown hard constraints or outcome data are not treated as zero or pass | Zero failed or not-evaluable candidates on the Pareto front in all test vectors | Negative-control fixtures and independent test oracle |
| B2: Pareto correctness | Multi-objective nondominated set matches brute-force reference calculation | 100% exact agreement on bounded exhaustive fixtures and randomized seeded fixtures | Compare engine front to simple independent oracle; do not compare scalar scores. Current goal: exhaust 729 combinations of three candidates × two discrete objectives. |
| B3: determinism | Identical input bytes and engine build emit identical canonical receipt bytes | 100/100 replays identical on a pinned build/platform | Repeat invocation and compare SHA-256 of output; include engine version and fixture digest in release evidence |
| B4: uncertainty honesty | Bounds are ordered, units consistent, and unknown outcomes remain explicitly unknown | 100% of invalid or missing intervals, unit mismatches, and negative or missing dominance tolerances rejected; 100% of unknown dimensions visibly surfaced | Adversarial fixture matrix plus report inspection |
| B5: reference performance | Generated synthetic benchmark with 10,000 candidate-scenario pairs and 8 objective dimensions | Initial target: p95 under 1 second on a documented fixed reference host after correctness gates pass | Use the Rust `portfolio-benchmark` binary; record CPU, OS, Rust version, exact command, input generator parameters, repetitions, p50/p95 and peak memory. This is a target to test—not a measured result or a universal requirement. |
| B6: transferability | Compare structurally different contexts without substituting a generic profile label for local data | At least four contrasting synthetic profiles; then independent site-specific datasets before real-world claims | Hold context assumptions apart; fail closed when local evidence is absent; publish null and adverse results |
| B7: real-world calibration | Simulated demand/resource/flow bands agree with held-out observations enough for the stated decision use | Pre-register model-specific error and interval-coverage targets with responsible domain experts before field assessment; report confidence intervals and segment/site failures | Separate calibration from held-out assessment; no universal pass number is defensible across queues, energy, water, resident outcomes and hazards |
| B8: real-world benefit | Visitor and investment choices do not hide burden shifted to workers, residents, local services or ecology | Every pilot reports absolute and per-visitor resource use, local retained-value measures, workforce/resident outcomes, access gaps and uncertainty; site-specific improvement goals must be agreed before launch | Baseline vs. counterfactual, seasonal adjustment, local review, transparent boundaries and evidence provenance |

## Important benchmark design choices

### Hard constraints are gates, not weighted objectives

No attendance, financial return, average satisfaction or index can cancel a safety, legal, rights, water or ecology violation. The engine initially only accepts `status = synthetic_fixture`; this is deliberate. Site-specific feasibility requires local evidence, accountable professional/authority review and a separate promotion process.

### No universal “good tourism” thresholds

A water-stressed site and a water-rich one cannot share the same unexamined water-use threshold. Cruise, alpine, urban and community-owned destinations differ in their boundaries and dependencies. Define limits from applicable law, local resource availability, ecological evidence and legitimate community decisions. Report both absolute use and intensity per visitor-night/visit; intensity can improve while absolute burden grows.

### Local value must be defined and observable

Where data permits, report the share of spend paid to locally owned suppliers, local wage flows/job quality, local SME participation, tourism value-added retained in the region, and the costs imposed on local services. Avoid presenting gross spending or a pledge as verified local benefit. Agree definitions and privacy protections with local stakeholders before measurement.

### Uncertainty benchmarks need held-out data

For a model that reports nominal 90% prediction intervals, one possible future pilot target is empirical coverage within 85–95% on a sufficiently large, pre-registered held-out sample per materially distinct context. This is a *candidate calibration target*, not a sector standard; account for dependence, seasonality and sample size. Require independent review before using forecasts to authorize any safety-critical operation.

## Current implementation scope

The Rust comparator reads the synthetic scenario manifest, validates paired candidate sets, hard-constraint states, units, per-objective dominance tolerances and uncertainty bounds, classifies candidates as infeasible/not-evaluable/feasible-under-declared-assumptions, aggregates a conservative cross-scenario envelope and emits a Pareto front without a weighted score. The test suite now includes a bounded exhaustive independent Pareto oracle across 729 inputs, all 81 combinations of four three-state hard constraints, and a generated scale benchmark that reports the exact input SHA-256 and p50/p95 timing samples. These tests have been authored but have not been run without Rust tooling.

The current milestone does **not** include a calibrated demand simulation, ecological carrying-capacity model, accessibility audit, market forecast, investment-grade financial model, or site-safety engine. These must remain separately qualified interfaces rather than invented data.
