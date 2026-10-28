# Paired resilience comparison protocol, v0.3

**Status:** proposed synthetic contract; not an executed controller experiment.  
**Evidence ceiling:** structural/evaluator correctness on authored fixtures. No physical model, real environmental trace, hardware-in-the-loop result, or controller superiority claim is made.

## Research contribution

The critical benchmark shift is from comparing aggregate scores produced by unrelated runs to a **paired, gate-by-gate controller comparison**. Each controller must be evaluated on every declared trajectory with the same initial-state identifier, hazard family, severity vector, replicate and comparison group. A missing execution must be represented as invalid or not evaluated; it may not disappear from the denominator.

The Rust evaluator requires a complete controller-by-trajectory matrix, validates signed gate margins against outcomes, reports every controller/gate coverage cell, and compares a declared reference controller with a candidate on the same trajectory IDs. Each gate must separately declare whether it is a hard gate, its margin unit, and the exact signed-margin definition; the current v0.3 evaluator accepts only hard gates. A positive margin means the declared gate criterion is met; a negative margin means a breach. Invalid and not-evaluated rows must have null margins. The synthetic fixture uses normalized unitless margins only to exercise contract logic; those numbers are not physical observations. One gate's surplus cannot compensate for another gate's breach.

The separate Python verifier independently recomputes the coverage and paired-outcome summary from raw input rows. It does not import Rust scoring logic. The workflow must run the Rust CLI first and verify that exact emitted report, followed by adversarial mutation tests. Agreement on a synthetic fixture is evaluator evidence, not physical validation.

## Required reporting

For every controller and gate, publish pass, breach, invalid, not-evaluated and valid-evaluated counts. For each paired gate, publish both-pass, both-breach, reference-breach/candidate-pass, reference-pass/candidate-breach and incomplete-pair counts, plus the distribution summary of candidate-minus-reference signed margin among rows with valid margins. Do not collapse these into a universal scalar.

Do not call a controller resilient because a gate has no observed breach when its coverage is zero. Do not treat incomplete pairs as passes. Do not infer probabilities from an unweighted stress set. Report the empirical stress suite and probabilistic risk separately; probabilistic estimates require an admitted sampling/probability basis.

## Research plan and measurable targets

1. **Evaluator integrity:** zero false passes across declared negative controls; reject missing controller/trajectory pairs, mismatched trajectory metadata, unsupported evidence escalation, inconsistent status/margin pairs, report tampering and unknown fields.
2. **Paired study:** freeze a baseline ladder and a locked whole-event/site-year holdout before tuning. Give all policies identical disturbances, controller-visible signals, action constraints, forecast horizon and simulator budget. Any additional information or compute becomes a separately reported resource premium.
3. **Outcome study:** predeclare critical-service floor violations as non-compensatory; report unserved service by type, deficit duration/severity, recovery time, state debt after repeated events, invalid-run rate, cost, runtime and resource consumption.
4. **Statistical study:** use the episode/site-year as the independent unit, preserve temporal blocks, publish paired effect estimates and confidence intervals, and account for multiple confirmatory comparisons. Do not set an achievable superiority percentage until a pilot estimates variability; keep the existing 10% CVaR95 target explicitly aspirational and not measured.
5. **Search study:** compare engineering-authored cases, seeded space-filling sampling, interaction coverage and adaptive falsification at fixed evaluation budgets. Score distinct independently replayed counterexamples per model call, time to first counterexample, replay success, novel failure-family yield and locked-holdout generalization. Search quality and controller quality are separate claims.
6. **Model validation:** start with one narrow, validated electrical subsystem; test the plant independently before controller rankings. Emit timestamped traces and conservation/solver diagnostics through a versioned simulator adapter. Add multi-site communications partitions only after delay, loss, stale data, fallback and state reconciliation semantics are explicit.

## Industry comparison without category errors

- **Microgrid control testing:** IEC TS 62898-3-5:2026 covers HIL, commissioning and periodic testing for monitoring, control and energy-management systems in grid-connected or isolated microgrids. Use its applicable licensed clauses and a reviewed clause-to-test crosswalk; this research contract is not a claim of IEC conformity.
- **Resilience metrics:** IEEE PES's 2026 report, *Power System Resilience Metrics and Evaluation Methods*, emphasizes selecting metrics suited to system level and operational context. Preserve service quantities, recovery, constraints and uncertainty rather than optimizing a single opaque score.
- **HIL/PHIL grounding:** NREL's microgrid-controller testbed work is a reference for closed-loop controller testing against real-time simulation. Synthetic trace scoring is below this evidence level.
- **Multi-domain coupling:** HELICS is an existing co-simulation option for connecting power, communications and other model domains; choose and validate each plant model instead of claiming Symtropy itself supplies validated physical truth.

This benchmark's potential differentiator is a versioned, fail-closed evidence and pairing layer that allows multiple external simulators and policies to be compared under an auditable protocol. It does not replace OpenDSS, REopt, HOMER, EMT models or hardware testbeds by assertion. Every comparison must document which tool is serving as optimizer, solver, plant simulator or reference.

## References checked for this protocol

- IEC, [IEC TS 62898-3-5:2026 — testing for microgrid monitoring, control and energy-management systems](https://webstore.iec.ch/en/publication/84175), published 11 June 2026.
- IEEE PES, [Power System Resilience Metrics and Evaluation Methods (TR143)](https://resourcecenter.ieee-pes.org/publications/technical-reports/pes_tr_143_amps_060826), published 8 June 2026.
- NREL, [Microgrid controller hardware-in-the-loop / power-hardware-in-the-loop test-bed report](https://www.nrel.gov/docs/fy20osti/71338.pdf).
- [HELICS documentation](https://docs.helics.org/en/latest/).
- NREL, [Power System Resilience Evaluation Framework and Metric Review](https://www.nrel.gov/docs/fy22osti/80817.pdf).

## Current state

The fixture is synthetic and encodes 12 result rows for two controllers across six trajectories, including valid passes, breaches, invalid runs and unevaluated outcomes. The six trajectories are not simulated physical systems. The Python verifier and its mutation suite check the report contract; only the exact-head GitHub workflow can compile and execute the Rust implementation. Even a passing workflow validates software contracts only. The remaining prerequisites for an empirical performance claim are real admitted inputs, an independently validated plant model, a frozen baseline and holdout, and paired experiments with uncertainty reporting.
