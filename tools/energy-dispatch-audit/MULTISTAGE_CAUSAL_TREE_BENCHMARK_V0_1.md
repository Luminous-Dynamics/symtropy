# Multi-stage causal-tree benchmark v0.1

## Purpose and qualification boundary

This benchmark tests whether a finite dispatch policy is implementable under the information actually available at each decision. It is a small exact oracle for causality and service-gap frontiers, not a physical dispatch solver, site model, safety certificate, or claim of superiority over industry tools.

The fixture is four futures by three intervals, with two delayed outage reveals. Scenario labels are evaluator metadata, never decision-time observations. The information key includes the full observable prefix, forecast values and availability times, previously committed actions, and battery SoC. Identical histories require identical actions; after an actual observation separates histories, different actions are permitted.

## Registered command

From the Symtropy repository root:

cargo test --manifest-path tools/energy-dispatch-audit/Cargo.toml --bin energy_multistage_causal_tree_oracle

cargo run --locked --manifest-path tools/energy-dispatch-audit/Cargo.toml --bin energy_multistage_causal_tree_oracle -- /tmp/energy-multistage-causal-tree-report.json

The binary embeds the frozen fixture at compile time and emits a report whose source SHA-256 must match the fixture file's SHA-256. The end-to-end workflow verifies that equality and preserves report, fixture, qualification log, and receipts as an artifact.

## Exact known answers

- Scenario-wise feasible schedules: 4, 7, 4, 2.
- Scenario-wise wait-and-see joint schedule profiles before enforcing information consistency: 224.
- Causal joint policies: 14.
- Objective order: worst-case essential service gap, worst-case flexible service gap, total essential service gap across futures, total flexible service gap across futures; all minimized.
- Causal Pareto frontier: [0,1,0,4], [1,0,1,0].
- Wait-and-see Pareto frontier: [0,1,0,1], [1,0,1,0].
- The service-first causal-frontier reference has a +3 flexible-unserved-quanta coordinate gap against the selected wait-and-see reference. This is a clairvoyance diagnostic, not a deployable improvement.
- No scenario probabilities are supplied; expected cost, EVPI, VSS, and CVaR are not computed.
- The physical dispatch audit status remains NOT_RUN for this abstract oracle.

## Fail-closed controls

The tests reject unavailable forecasts, attached unsupported probabilities, duplicate or reordered scenario identities, missing/reordered intervals, altered action catalogs, and infeasible grid-connected battery service. They also check a delayed-reveal recourse policy exists, so a gate that freezes all future actions would fail the oracle.

## Industry comparison ladder

Do not claim “better than SOTA” from this fixture. Compare in layers and keep inputs, horizons, fidelity, hardware, solver versions and budgets matched:

1. Exact correctness: this finite policy oracle, mutation suite, and zero false-feasible cases in the separate independent dispatch auditor.
2. Causal control: held-out outage/demand/forecast timing families; policy quality, feasibility, constraint violations, false-safe rate, and worst-case essential service.
3. Physical dispatch: same requests through Sol-Atlas and independent audits; voltage/network constraints and time coverage where supported.
4. Facility planning: matched REopt/SAM cases for sizing, cost, storage dispatch, degradation and replacement.
5. Power systems: public IEEE PES Power Grid Lib OPF and PGLib-UC cases, reporting optimality gap, feasibility, runtime and memory.
6. Energy model/solver performance: use Open Energy Benchmark-style standardized settings across model frameworks and solvers; do not infer one universal best solver.

Reference documentation: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/, https://developer.nrel.gov/docs/energy-optimization/reopt/, https://sam.nrel.gov/battery-storage.html, https://github.com/power-grid-lib/pglib-opf, https://github.com/power-grid-lib/pglib-uc, and https://openenergybenchmark.org/.

## Promotion criteria

A production/industry claim needs a Rust implementation and this exact oracle, independent physical feasibility auditing, mutation results on registered negative controls, held-out scenario families, lockfile/commit digests, and benchmark receipts. No gate is passed by queued, and no physics or economic claim is inferred from an abstract one-quantum model.
