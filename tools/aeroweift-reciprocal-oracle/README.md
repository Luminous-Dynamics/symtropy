# AeroWeft Reciprocal Option Oracle — Synthetic Slice 0.1

This dependency-free Rust workspace is a **tiny exhaustive reference oracle** for the hand-solvable two-aircraft counterexample in:

- Research issue: https://github.com/Luminous-Dynamics/symtropy/issues/5207
- Research specification and fixture: https://github.com/Luminous-Dynamics/symtropy/pull/5209
- Golden JSON: fixtures/aeroweift-reciprocal-options/tiny-option-externality-v0.json

## What it evaluates

- Two synthetic aircraft with fixed, frozen route domains.
- Two admissible weather branches whose initial delivered observation is identical.
- Half-open integer-tick claims on three capacity resources.
- Whether each route meets its deadline on every declared branch.
- Whether a pair of routes jointly fits every shared resource.
- Exhaustive enumeration of the complete two-aircraft assignment space.
- The constrained aircraft's robust route-option count before and after a paired allocation intervention.
- A negative control that changes route based on hidden weather branch identity despite identical observations.
- Fail-closed handling of unknown resources and invalid claim intervals.

The fixture-local result is:

- Allocate the shared crossing to aircraft A: aircraft B has zero robust route options.
- Move aircraft A to its wider bypass: aircraft B has one robust route option.
- The only complete joint assignment in the frozen finite domain is A-bypass plus B-crossing.
- A policy that chooses a different route for A using the hidden weather branch is non-anticipative-invalid.

The count difference is a synthetic finite-domain measurement, not a physical safety margin and not comparable across aircraft with different option domains.

## Deliberate limitations

This is not a flight-dynamics model, aircraft-performance claim, continuous-time reachability/viability solver, production policy-tree evaluator, fleet-scale scheduler/optimizer, probabilistic weather/risk model, live UTM/ATC service, dispatch system, flight-clearance service, or aircraft-control component.

It does not establish that a real route is physically feasible or safe. It is a reference answer for the exact finite fixture only. Future adapters should consume the existing Symthaea option-set and policy-tree contracts instead of growing this test oracle into a second generic engine.

## Exact-head commands

cargo fmt --manifest-path tools/aeroweift-reciprocal-oracle/Cargo.toml -- --check

cargo clippy --locked --manifest-path tools/aeroweift-reciprocal-oracle/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path tools/aeroweift-reciprocal-oracle/Cargo.toml

The source is authored; no test pass is claimed until the exact-head workflow completes.
