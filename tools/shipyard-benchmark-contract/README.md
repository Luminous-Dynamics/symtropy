# Shipyard benchmark contract (v0)

This directory defines a small, versioned interchange contract for reproducible shipyard-placement, disruption-recovery, and execution-replay experiments. It is a protocol and schema proposal, not an optimizer, a safety certificate, or evidence that any benchmark has passed.

## Why this is the next step

The current stack has four kinds of evidence that should never be collapsed into one score:

1. **Semantic correctness:** exact tiny-instance enumeration and independent reference checkers can falsify a candidate and detect false-feasible / false-infeasible outcomes.
2. **Algorithmic quality:** on the same frozen cases and budgets, compare feasible coverage, objective vectors, regret against a known bound, and Pareto trade-offs.
3. **Operational validity:** execution replay must compare planned windows with timestamped observations, preserve as-of cutoffs, and report missing telemetry separately from schedule failure.
4. **Industrial validation:** only source-authorized yard traces and matched baselines can support claims about actual travel, re-handles, utilization, or delivery performance.

A high score in one class cannot substitute for evidence in another. Zero errors on generated rectangles is not a crane-safety result, and a fast optimizer is not an industrial productivity result.

## Files

- `benchmark-manifest.schema.json`: protocol inputs, source identity, model scope, solver budgets, candidate and baseline revisions, reference-oracle identity, and declared metrics.
- `run-report.schema.json`: per-case outcomes, verifier results, run provenance, and summary counts. Unknown, timeout, invalid-output, and not-run are explicit outcomes rather than silently omitted cases; the report summary exposes separate counts for not-run and invalid-output cases.

The schemas use JSON Schema Draft 2020-12. The manifest pins the expected case count and explicit metric definitions (unit, formula, aggregation, direction, and optional weight), not merely metric names. Each report case has a separate reference outcome so false-feasible and false-infeasible counts can be derived independently. Implementations should validate the manifest before running and validate the report before publishing it. A schema-valid report can still contain bad science; its hashes and claims require review.

## Minimum run protocol

1. Freeze the case corpus or generator recipe, its parameters, seed, units, time semantics, and source digest. Use half-open intervals consistently where the underlying model specifies them.
2. Freeze candidate and baseline commit IDs, configuration digests, model scope, objective definitions, time limit, candidate-evaluation budget, hardware class, repetitions, and timeout policy **before** observing results.
3. Give every baseline the same per-instance budget and the same authoritative verifier. Separate optimization runtime from input parsing, verifier runtime, and report serialization.
4. Save every case result. A timeout is not infeasibility; heuristic exhaustion is not infeasibility; a verifier rejection is not an optimizer success.
5. Publish per-instance records and aggregate statistics. For stochastic methods, use a predeclared seed list and enough repetitions to report dispersion; when reproducing a published protocol, retain its repetition count rather than cherry-picking best runs.
6. Keep metrics separate: feasibility errors; feasible coverage; area-time utilization; travel; makespan/tardiness; recovery horizon; affected and unaffected churn; re-handles; regret/bound gap; runtime; and peak memory. Do not create a single weighted score unless weights, sensitivity, and decision rationale are published.
7. Record every unsupported constraint explicitly. A model must not silently treat an absent polygon, crane capacity, swept volume, lift qualification, cancellation, or source event as unconstrained free capacity.

## Comparison ladder

- **Gate A — semantic:** exact oracle agreement for every case within the declared tiny domain, zero false-feasible and zero false-infeasible, including negative controls. Report budget exhaustion as unknown.
- **Gate B — heuristic:** publish recall over exactly feasible cases, false-positive/false-negative counts, runtime distributions, and which instances each policy misses. Include counterexamples that defeat the heuristic.
- **Gate C — recovery:** test disruption sequences, as-of observations, operation immutability, non-anticipativity, outage constraints, and Pareto vectors. A candidate that fails any declared hard scenario cannot enter the robust frontier.
- **Gate D — published reproduction:** reproduce the same instance geometry, process order, disruption schedule, objective weights, inputs, hardware, and termination rules before comparing headline numbers.
- **Gate E — industry:** compare a permissioned and source-pinned execution trace against a declared baseline; include data completeness, clock/recording latency, and confidence intervals. Report privacy/access restrictions without publishing protected raw data.

## Metrics and interpretation

- **Correctness:** false-feasible, false-infeasible, verifier/oracle agreement, unknowns, timeouts, invalid outputs.
- **Space and flow:** area-time utilization, travel time/distance, makespan, tardiness, waiting, and re-handles.
- **Recovery quality:** recovery horizon, schedule regret, reservation changes, unaffected-plan churn, time shift, geometric displacement, and robust-scenario pass fraction.
- **Execution fidelity:** start/finish deviation, due-but-missing observations, recording latency, and missing-event counts.
- **Efficiency:** solver wall time, verifier time, peak resident memory, evaluations, and bound gap when a valid bound exists.

Every metric needs a declared unit and calculation definition in the benchmark description; metric names alone are not sufficient to compare studies. Report unsupported metrics as absent, not as zero.

## Initial published comparators

- Wang, Yin, and Khan, *Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding*, first published 11 July 2026, [DOI 10.1049/cim2.70066](https://doi.org/10.1049/cim2.70066). This is the first reproduction target for irregular-block geometry and disruption recovery, but the current rectangular baseline is not yet a reproduction.
- *An efficient multi-fidelity ant colony system for multi-crane scheduling optimization*, *Applied Soft Computing* 200 (August 2026), [DOI 10.1016/j.asoc.2026.115384](https://doi.org/10.1016/j.asoc.2026.115384). This is a later comparator after the model represents safety separation, interference, precedence, and tandem operations. Unary resource reservations are not enough for a fair claim against it.
- *Digitalisation of Shipyard Production Planning: A Review of Simulation, Optimisation, AI, and Digital Twin Methods (2010–2025)*, [DOI 10.3390/jmse14040396](https://doi.org/10.3390/jmse14040396). The review supports prioritizing hybrid simulation/optimization and measured execution KPIs over architectural claims unsupported by longitudinal validation.

## Non-negotiable claim rules

- A passing schema is not passing tests, and passing tiny-instance oracles is not industrial validation.
- Never label queued checks as passed. Store the exact tested commit SHA in the published report.
- No SOTA claim unless the baseline is matched by instance, objective, constraints, compute budget, hardware, and termination criterion.
- Report both improvements and regressions, distributions rather than only best runs, and any excluded/time-out instances.
- Treat provenance hashes as integrity references, not proof that source data are true, complete, authorized, or representative.
