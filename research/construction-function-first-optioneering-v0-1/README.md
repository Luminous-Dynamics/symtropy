# Construction Function-First Optioneering Core — v0.1

**Status:** synthetic, evidence-gated robust Pareto evaluator for a finite construction option catalogue. Not a design verifier, safety authority, professional engineering opinion or live-project recommendation.

Parent: [CONSTRUCTION-FUNCTION-FIRST-OPTIONEERING-001 #5143](https://github.com/Luminous-Dynamics/symtropy/issues/5143).

## Paradigm

Evaluate whether the same functional outcome can be achieved by:
1. **Avoid** — reuse/adapt an existing asset or remove an unnecessary intervention while preserving the function;
2. **Switch** — change design, materials, prefabrication or delivery method;
3. **Improve** — optimize the chosen method's resources and execution.

The tier is descriptive, not an automatic preference. An option only enters the Pareto comparison if every declared hard gate has qualified-pass evidence.

## Implemented v0.1

- Functional brief pins exact revision, required hard gates, explicit scenarios, analysis horizon, discount-rate assumption, profile/fixture references and claim ceiling.
- Every option binds to the same functional brief and design revision.
- Hard gate results are `QualifiedPass`, `QualifiedFail`, `Unknown` or `Stale`.
- Failed options are `Infeasible`; unknown/stale options are `NotEvaluable`; only all-pass options are `QualifiedFeasible`.
- Required gates must be complete and no undeclared gate is silently ignored.
- Every option has a complete metric vector for every declared scenario.
- Metrics are distinct and unit-typed by `MetricKey`; directions are explicit (lower or higher is better).
- Worst-case values are computed across all scenarios: maximum for lower-is-better metrics and minimum for higher-is-better metrics.
- Robust Pareto dominance compares the whole worst-case metric vector. No probabilities, scalar weights or hidden objective score are introduced.
- Dominated options list their exact robust dominators; equal options remain tied on the frontier.
- Reports preserve brief/profile/fixture identity and claim ceiling.

## Local commands

```sh
cargo fmt --manifest-path research/construction-function-first-optioneering-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-function-first-optioneering-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-function-first-optioneering-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

## Important boundaries

The code only consumes declared gate status and evidence references. It does not establish structural adequacy, fire safety, accessibility, environmental compliance, permissions, cost rates, emissions factors or professional acceptance.

Scenario values are synthetic fixture inputs, not forecasts or probabilities. The whole-life cost metric must be supplied with a declared analysis horizon and assumptions; this crate does not discount cashflows or invent rates. A qualified option means only that the fixture's required gates were marked passed, not that a real design is valid.

No aggregate scalar “best option” is emitted. The output is a robust Pareto set, worst-case metric vectors and explicit infeasible/not-evaluable dispositions.

## Claim ceiling

V0.1 establishes only deterministic synthetic hard-gate classification, worst-case metric aggregation and robust Pareto dominance for a finite catalogue. It does not establish real savings, environmental benefits, structural safety, regulatory compliance, or superiority to P6, ALICE, SYNCHRO, Navisworks or any other commercial system.
