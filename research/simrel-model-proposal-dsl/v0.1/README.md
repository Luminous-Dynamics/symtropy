# SIM-REL model proposal DSL v0.1

This isolated, dependency-free Rust crate is the first executable proposal language for Symtropy issue #6027.

## Claim ceiling

The interpreter accepts only a closed enum AST. It does not run arbitrary source text, shell commands, native plugins or dynamically loaded code. A validated program is executed with explicit variable declarations, dimensions, bounds, a maximum horizon and a finite expression-fuel budget.

The schema currently supports three proposal classes:
1. Algebraic: static causal and measurement maps.
2. Discrete state: synchronous state-transition maps.
3. Stochastic state: synchronous state-transition maps with seed/step/channel-keyed standard-normal innovations.

This is a finite model language, not a universal scientific language. It currently does not support continuous-time ODE/DAE/PDE/delay operators, arbitrary event systems, non-Gaussian stochastic distributions, temporal input schedules, loops, recursion, arbitrary functions, foreign code or unconstrained memory allocation.

## Core execution contract

- Every variable has a stable numeric ID, constrained ASCII name, role, dimension and optional admissible range. Dynamic states also have a required initial value.
- Addition and subtraction require identical dimensions. Multiplication and division compose integer dimension exponents. Exponential arguments must be dimensionless.
- Every observable is assigned once; every state/latent variable in a stateful proposal is updated exactly once. Algebraic programs may not smuggle in hidden state.
- All state transitions in a step read one frozen pre-step environment. They are staged, range-checked and committed together. Outputs are sampled before the transition.
- Input and parameter bindings are explicit, unique and range-checked.
- Stochastic draws are keyed by (seed, step, channel ID). A repeated channel ID denotes the same draw within a step; different IDs denote distinct coordinates.
- AST depth, node count, program size, horizon and runtime expression fuel are bounded.
- Canonical serialization sorts declarations and assignments by stable IDs. The FNV-1a-64 fingerprint is for reproducibility only and is not cryptographic; do not use it as authenticity evidence.

## Benchmark tests

The unit suite covers an affine algebraic sensor map, a hand-computed discrete trajectory, seeded stochastic reproducibility, canonical fingerprint invariance under declaration/assignment ordering, unit mismatch, out-of-range bindings, duplicate/missing/unknown bindings, stochastic primitive admission, no hidden state in algebraic proposals, complete state updates and fuel/horizon exhaustion.

The code is synthetic-only. It is not a physical model, truth oracle, state estimator, parameter fitter, model-selection authority or evidence/security boundary.

## Future qualification gates

1. Add property tests and fuzzing for parser/AST validation and arithmetic failure paths.
2. Add exact rational/small-state interpreters as independent oracles for the discrete subset.
3. Add typed intervention schedules with event/availability time and historical-cutoff semantics.
4. Add explicit non-Gaussian noise families only with separate process identities and calibration.
5. Add continuous-time operators only when compared against independent analytic and numerical references.
6. Add evaluator-side sealed owner generators and counterexample minimization in a separate process; candidate proposals must never see hidden truth or evaluator seeds.
7. Report structural discovery, parameter fitting, sensor-model recovery and model adequacy separately.