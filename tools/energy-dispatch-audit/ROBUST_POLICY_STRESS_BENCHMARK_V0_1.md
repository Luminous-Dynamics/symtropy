# Robust policy stress benchmark v0.1

## Status

This file and its fixture are research inputs for a future Rust implementation. They do not define a passing production gate by themselves. The Python standard-library reference lab lives in the companion research artifact and is not part of the production path.

## Research hypothesis

A policy that reserves storage against the maximum essential-load requirement among futures consistent with its observable history can reduce essential unserved demand under a finite, explicit uncertainty set. This should be measured against greedy essential-first, greedy flexible-first, and conservative hold-reserve baselines. The trade-off against flexible service must remain visible in the full metric vector.

## Registered policy

At each step:
1. Read current observable grid, essential demand, flexible demand, time, SoC, and the observed history. Do not branch on evaluator-only scenario ID or inspect the realized future.
2. Match the observable prefix against the frozen training scenario set.
3. Calculate the maximum future essential energy requirement across matched scenarios, respecting the per-interval discharge limit and treating grid-served demand as not requiring battery reserve.
4. Serve immediate essential demand if feasible. Serve flexible demand only if a one-quantum discharge leaves at least the required reserve. If no registered scenario matches the observed prefix, preserve full capacity as reserve and deny discretionary flexible service.
5. Emit decisions, SoC transition, reserve, unserved demand, and prefix-match receipt for every scenario/time pair.

The method is not robust against every possible real-world future. The held-out scenarios deliberately test out-of-set behavior and must remain distinct from training scenarios. An out-of-set prefix is a coverage alarm, not evidence that a future distribution has a known probability.

## Frozen panel

Four three-interval training scenarios and four three-interval held-out stress scenarios are registered. Four reference controller baselines are required. The local Python standard-library reference ran 16 tests successfully.

On this particular synthetic held-out panel:
- set-valued essential reserve: total/worst essential unserved = 3/1 quanta; total/worst flexible unserved = 7/3;
- greedy essential-first: total/worst essential unserved = 5/2 quanta; total/worst flexible unserved = 4/2;
- the set-valued controller records 9 training-prefix misses across held-out decisions and fails closed for flexible service when its registered set does not cover the current prefix.

This observation supports a testable hypothesis, not a statistical guarantee or an industry result.

## Acceptance criteria for the Rust port

- Exact agreement with the local reference for the frozen decisions and metric vectors.
- Fail-closed controls for missing, duplicate, reordered, malformed, or forged scenario histories; invalid demand; unsupported probabilities; unavailable forecasts; and out-of-set prefixes.
- No false-feasible outcomes when coupled to the independent physical dispatch auditor on registered mutation controls.
- Full service-contract Pareto vectors; no arbitrary scalar weight hiding essential-versus-flexible trade-offs.
- Separate training and held-out evaluation, exact source/report SHA-256 digests, complete scenario × interval accounting, runtime, memory, and solver/build identifiers.

## Industry comparison ladder

Use a layered comparison with matched model fidelity, horizon, constraints, resource budgets, and solver versions:
1. Exact finite causal-policy oracle and negative controls.
2. Held-out scenario shifts, policy regret, essential-load guarantee, flexible-load shedding, uncertainty-set coverage, and runtime.
3. Physical dispatch and network constraints through the independent Sol-Atlas auditor where supported.
4. Facility size/cost/resilience comparisons with REopt/SAM-like cases, including degradation when represented.
5. Power systems: IEEE PES Power Grid Lib OPF and PGLib-UC; report feasibility, optimality gap, runtime and memory.
6. Solver comparisons under Open Energy Benchmark-style standardized settings; do not claim one universal best solver.

Public references: PyPSA stochastic optimization (https://docs.pypsa.org/latest/user-guide/optimization/stochastic/), NLR REopt (https://developer.nlr.gov/docs/energy-optimization/reopt/), Open Energy Benchmark (https://openenergybenchmark.org/), 2025 benchmark results published 28 May 2026 (https://www.openenergytransition.org/posts/the-state-of-solvers-for-energy-planning---results-of-the-2025-open-energy-benchmark), PGLib-OPF (https://github.com/power-grid-lib/pglib-opf), PGLib-UC (https://github.com/power-grid-lib/pglib-uc), and a 2025 review on uncertainty in hybrid-microgrid EMS (https://ietresearch.onlinelibrary.wiley.com/doi/full/10.1049/rpg2.70174).

## Claim boundary

No evidence-backed scenario probabilities are supplied. Do not calculate expected cost, EVPI, VSS, CVaR, confidence intervals, or a real-site safety claim from this fixture. A full SOTA claim requires matched public cases and independent reproducible runs.
