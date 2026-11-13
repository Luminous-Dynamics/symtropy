# Matched-semantics multibody dynamics benchmark v0.1

Starts [PHYS-INDUSTRY-BENCH-001 #5080](https://github.com/Luminous-Dynamics/symtropy/issues/5080).

## Purpose

This contract is designed to compare Symtropy against MuJoCo, Drake, and Pinocchio without changing the physical problem or forcing an engine to use an algorithm that is not native to it.

## Admission before timing

Each engine run binds to the same model/topology, inertial parameters, initial state, force history, constraint semantics, units, integrator configuration, and observable contract. A comparison is not `MATCHED_COMPARISON` unless these semantic signatures match exactly.

Timing requires at least 30 raw samples and records the full hardware/compiler/configuration. The schema retains physical errors and rank/constraint status alongside latency. Performance cannot be reported as SOTA or even a speed win without matched semantics and independent correctness evidence.

## Industry context

- MuJoCo documents Gauss' least-constraint principle in the inertia-weighted norm, its Delassus operator, and an effective metric for the discrete integrator. See [computation docs](https://mujoco.readthedocs.io/en/latest/computation.html).
- Drake documents O(n) recursive Newton-Euler inverse dynamics and an O(n²) Composite Body Algorithm for explicit mass matrix construction, and warns against forming M when unnecessary. See [MultibodyPlant docs](https://drake.mit.edu/pydrake/pydrake.multibody.plant.html).
- Pinocchio is a mature articulated-body dynamics and derivative library; its adapter must preserve native articulated algorithms rather than explicitly build dense M just to fit our reference.

These systems are baselines, not strawmen. Symtropy's dense reference is for correctness and small fixed-size oracle cases; articulated scaling must be benchmarked using efficient native algorithms.

## Fixture

`fixtures/synthetic-6dof-bundle-v0.1.json` is a shape-only example. Its run metadata and timing samples are synthetic placeholders. It is not an admissible benchmark result.

## Next steps

1. Freeze a real 6-DOF constrained fixture and independent high-precision oracle.
2. Implement thin adapters for MuJoCo, Drake and Pinocchio.
3. Prove equality of semantic signatures and compare physical outputs before timing.
4. Scale to 6, 12, 24, 48, 96 and 192 DOF; separate unconstrained, hard-constraint, soft-constraint and contact strata.
5. Publish raw samples, medians/p95/p99, memory, error envelopes, rank status and all failures.
