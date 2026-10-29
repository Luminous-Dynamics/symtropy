# Symtropy counterexample-search benchmark protocol

**Status:** design-only / proposed, not executed.  
**Current evidence:** the contract and validator are specifications. No physical oracle, controller, scenario-searcher comparison, or measured discovery rate is admitted.

## Why this is a separate benchmark

A controller evaluation is only as informative as the scenarios presented to it. A searcher may report few failures because the controller is robust—or because the searcher is inefficient, repeatedly visits the same regions, labels invalid runs as safe, or never exercises the critical interaction. Therefore controller performance and searcher performance must be measured separately.

This protocol compares methods for finding **independently replay-confirmed counterexamples** at equal expensive-oracle budgets. It does not infer controller safety from zero discoveries and does not turn synthetic oracle results into physical evidence.

## Searcher arms

The initial baseline ladder is:

1. **Uniform random sampling.** Simple, reproducible reference.
2. **Deterministic space-filling.** A predeclared sequence (for example, a low-discrepancy design) to reduce avoidable sampling clumps.
3. **Single-objective robustness optimization.** A formal-specification falsifier such as a compatible S-TaLiRo/Breach adapter, when the model interface permits a fair comparison.
4. **Coverage-guided search.** Select inputs to exercise uncovered requirements, transition regions or behavior partitions.
5. **Symtropy adaptive multi-objective candidate.** Search across hard-gate margins, discovered failure-family novelty and uncertainty, while maintaining independent replay and admission rules.

These are method classes, not claims that all adapters are currently implemented. Pin exact implementations and versions before the confirmatory study. Add a method only before holdout lock, with a documented compatibility rationale. Do not remove a weak-looking baseline after seeing its result.

## Fixed budget and equal information

The proposed pilot uses 10 independent search runs per arm at 50 oracle invocations per run. The confirmatory stage uses 50 independently seeded runs per arm at 200 oracle invocations per run. These are proposed resource limits; they are not observed performance results. Freeze the final limits and all seeds before running the locked holdout.

- Every searcher gets the same domain, formal property, gate definitions, controller observation contract, plant model and numeric resource limits.
- Every oracle invocation consumes budget, including invalid inputs, simulation timeouts, failed executions, replay calls, surrogate-training queries and cache misses. Cached results must be content-addressed; no searcher may receive unbudgeted pretraining queries.
- Candidate generation may use a method's own past, non-holdout observations, but must not inspect future disturbances or holdout cases.
- Randomize or block run order on the same hardware/software environment. Record CPU time, wall time, memory and resource use separately from oracle-call count.
- Repetitions are independent at the search-run level. A single lucky seed is not an empirical result.

## Counterexample admission

A violation candidate only becomes a confirmed counterexample after an independent replay under the exact input digest, pinned controller artifact and pinned plant-model artifact reproduces the same formal gate/failure family. Failed, invalid or unevaluated runs never count as discovered violations. Identical rediscoveries count once for unique failure-family yield, but their invocation cost still counts.

Version the failure-family fingerprinting policy before the confirmatory set. If two traces have the same fingerprint under the frozen policy, count them as one family and preserve both trace digests. If the fingerprint policy changes, treat the new results as a different benchmark version.

## Metrics and statistical analysis

The primary endpoint is **falsification rate at the confirmatory invocation budget**: the proportion of independent runs that find at least one replay-confirmed counterexample by the budget limit.

Report, without collapsing into a single composite score:

- Falsification rate with confidence intervals.
- Survival curve for runs that have not yet falsified; censor at the fixed invocation budget.
- Invocation count to first replay-confirmed counterexample, accompanied by falsification rate so a few early successes cannot mask a method that mostly fails.
- Unique replay-confirmed failure families per 100 oracle invocations; discovery yield at 25, 50, 100 and 200 invocations.
- Invalid/timeout rate, duplicate failure-family rate, replay confirmation rate, per-gate discovery and locked-holdout failure-family yield.
- Cross-seed variability and resource use.

Use paired or blocked seeds when the design supports it, analyze independent search runs, account for multiple confirmatory comparisons and predeclare the primary endpoint. Keep the relative superiority target unset during the pilot; freeze a practically meaningful effect size before unlocking the holdout. Superiority requires a paired uncertainty interval that excludes zero and no unacceptable regression on replay reliability, gate coverage or resource budget.

## Three evidence tracks

1. **Analytic known-answer track:** synthetic functions with exact, known failure regions. Use it to debug the search/measurement/replay plumbing and assess known-region discovery. It is not physical validation.
2. **Admitted plant-model track:** an independently verified plant model with reviewed formal service constraints, numerical checks and pinned simulator. This track is currently not admitted.
3. **Locked site/episode track:** distinct scenario families, episodes or site-years that are sealed before confirmatory tuning. This track has not been created; source provenance, data-use permissions and model validity remain prerequisites.

Report generalization to the locked track separately. Do not tune on the locked set and then reuse it as a test. Do not describe a synthetic benchmark victory as SOTA over physical controllers.

## Industry and research comparison

This design follows established criteria for comparing stochastic falsification methods. The 2025 study *Requirement falsification for cyber-physical systems using generative models* evaluates repeated runs using falsification rate, execution budget and survival analysis; it explains why time-to-first-failure must be interpreted alongside the probability of finding any counterexample. The 2025 paper *Automated Generation of Benchmarks for Falsification of STL Specifications* addresses the lack of varied benchmarks for assessing falsifiers. A 2026 paper on standardized benchmarking of search-based scenario-selection methods likewise argues for domain-independent interfaces, controlled synthetic instances and reproducible evaluation protocols.

The potential Symtropy contribution is not a novel score by itself. It is the combination of strict receipt-bound executions, equal-information/equal-budget comparison, independently replayed counterexamples, failure-family diversity, locked-site generalization and explicit separation between searcher quality and controller quality.

References:
- [Requirement falsification for cyber-physical systems using generative models](https://link.springer.com/article/10.1007/s10515-025-00503-x), *Automated Software Engineering*.
- [Automated Generation of Benchmarks for Falsification of STL Specifications](https://doi.org/10.1109/TCAD.2025.3550410), *IEEE Transactions on Computer-Aided Design of Integrated Circuits and Systems*, 2025.
- [Toward Standardized Benchmarking of Search-Based Scenario Selection Methods in Autonomous System Validation](https://doi.org/10.3390/jmse14151386), *Journal of Marine Science and Engineering*, 2026.
- [Continuous Requirement Conformance Falsification via Cross-Revision Knowledge Transfer](https://conf.researchr.org/details/icst-2026/icst-2026-research/29/Continuous-Requirement-Conformance-Falsification-for-Cyber), ICST 2026; future research opportunity for controlled reuse across revisions.

## Executable analytic known-answer smoke track (new, synthetic only)

The repository now includes an executable Rust smoke track at `src/bin/analytic_counterexample_track.rs`, an independently authored seven-case golden fixture at `fixtures/analytic-counterexample-known-answer-2026.json`, and strict schemas for the fixture and report. The paired-comparison workflow tests and runs the track on the exact checked-out revision.

The known-answer oracle has two disjoint failure families in a unit-square domain:
- Critical-energy gate: breach when `x + y > 1.4`; signed margin `1.4 - x - y`.
- Communications-partition gate: breach when `x < 0.2` and `y > 0.7`; signed margin `max(x - 0.2, 0.7 - y)`.
- Boundary equality is a pass. The separate golden fixture includes boundary-pass cases, just-over-boundary breaches, one safe interior point and points in both failure families. Rust tests compare oracle outputs to this separately authored fixture before emitting a report.

Four arms exercise the fixed-budget machinery: seeded uniform random, Halton space filling, stratified grid, and an explicitly labelled energy-only diagnostic control. Each arm gets 38 search queries plus exactly two replay/positive-control calls, for 40 total oracle invocations. Unused replay slots run the known-safe positive control; they are not silently spent on extra search calls. The grid is expected to locate and replay-confirm both analytic families, while the energy-only diagnostic should find only the energy family. This is a known-answer test of coverage bookkeeping and replay controls, not evidence that the grid is a better general search algorithm.

The emitted report fixes `physical_claim=false`, `controller_superiority_claim=false`, and `industry_sota_claim=false`. No statistical comparison is inferred from this single finite toy suite. The production benchmark still needs compatible, pinned implementations of the five planned searcher classes, an admitted physical oracle, repeated independently seeded runs, exact counterexample fingerprints, independent replay, and a sealed holdout.

## Current implementation status

- Added a machine-readable fixed-budget contract and strict Draft 2020-12 schema.
- Added a Rust fail-closed contract validator and negative controls for unfair budgets, holdout leakage, non-replayed failures, unadmitted evidence and composite-score claims.
- The contract remains design-only with all admission gates false. The analytic known-answer smoke track now exercises four simple deterministic search arms, fixed-budget replay and a seven-case golden fixture. It does not implement the full five-arm production comparison and does not admit a physical model.
- The next scientific milestone is to pin compatible production searcher adapters and independent replay on the analytic track, then apply them to an independently admitted plant model with repeated runs and a locked holdout.
