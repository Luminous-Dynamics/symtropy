# Matched metric-physics benchmark contract v0.1

This is the first machine-readable contract slice of [PHYS-METRIC-BENCHMARK-001 #4805](https://github.com/Luminous-Dynamics/symtropy/issues/4805). It pins one frozen constrained-mechanics case and defines when results from different implementations may be compared.

## Pinned physical case

The benchmark binds the exact [DUMBBELL-DENSE-REBASIS-001 fixture](https://github.com/Luminous-Dynamics/symtropy/blob/b9fcf688b502d602eac91297880bf3a3e484545f/research/generalized-coordinate-covariance-001/fixtures/covariance_case_v0.1.json) by commit and Git blob SHA. It is a 3D two-point-mass fixed-link model with unequal masses (1.3 kg, 2.1 kg), an active nonzero constraint reaction, a 0.02 s constrained midpoint step, and two dense unimodular coordinate rebases. The transformed mass-metric condition numbers are approximately 2.06e5 and 6.02e4.

The benchmark manifest does not retype the full initial state and matrix data. It pins the canonical case bytes and schema, avoiding a second divergent copy of the physical fixture.

## Matched-comparison gate

Drake, MuJoCo and Pinocchio may only be compared when the adapter records and verifies the same:
- physical model and generalized-coordinate ordering;
- velocity and force/covector conventions;
- constraint equations and initial state;
- timestep and integration method;
- units and solver tolerances.

If any required mapping is missing or different, the result must be `NOT_COMPARABLE`. This is not a failed solver run; it is a refusal to rank unlike experiments.

## Separate metric groups

1. Semantic mapping and fixture identity.
2. Coordinate invariance: tangent/cotangent norm drift, dual pairing, metric congruence, constraint-Jacobian transformation, and raw coordinate-norm variability.
3. Numerical quality: backward error, metric-induced operator condition, forward-error envelope and independent high-precision coverage.
4. Physical fidelity: lifted state, momentum, constraint, energy/work and discrete-step defects.
5. Decision quality: false accepts/rejects, negative controls and inconclusive coverage.
6. Systems cost: solve/factorization latency, evidence-layer overhead and memory.

These groups must not be collapsed into one score. In particular, latency is meaningful only after semantic matching; low residual is not physical accuracy; and coordinate-infinity norm variability must remain visible beside metric-induced invariance.

## Industry mapping

- **Drake:** map the same model and compare the mass-matrix and constrained-step semantics. Drake documents Composite Body Algorithm mass-matrix construction and cautions against explicit mass-matrix construction when avoidable. Record algorithm path and solver configuration; do not compare a dense reference kernel's time to recursive production dynamics as if they were equivalent.
- **MuJoCo:** document the exact model conversion, generalized coordinates, inertia representation, constraint formulation and solver. Its computation documentation describes inertia-weighted Gauss-principle constrained acceleration, sparse factorizations and Newton/CG/PGS solvers; these are strong baselines, not a reason to equate unlike soft/hard constraint formulations.
- **Pinocchio:** map the model using its spatial-algebra conventions and record whether CRBA, ABA, RNEA or constrained algorithms are being timed. Its recursive algorithms exploit kinematic-tree sparsity; dense 6x6 microbenchmarks do not establish superiority over articulated-body algorithms.

Official references:
- [Drake MultibodyPlant documentation](https://drake.mit.edu/pydrake/pydrake.multibody.plant.html)
- [MuJoCo computation and solver documentation](https://mujoco.readthedocs.io/en/latest/computation/)
- [Pinocchio dynamics algorithms](https://gepettoweb.laas.fr/doc/stack-of-tasks/pinocchio/master/doxygen-html/md_doc_a-features_g-dynamic.html)
- [Higham, Accuracy and Stability of Numerical Algorithms, Chapter 7](https://epubs.siam.org/doi/10.1137/1.9780898718027.ch7)

## Prospective benchmark targets

- 100% exact agreement on the frozen transformation identities.
- Zero false physical ACCEPT decisions across wrong-variance, wrong-congruence, wrong-force, wrong-Jacobian, stale-fixture and altered-constraint controls.
- 100% explicit INCONCLUSIVE on singular, non-finite or out-of-profile inputs.
- Every admitted forward-error envelope covers the independent high-precision observed error.
- Metric-induced norm drift stays within a preregistered condition-tier envelope across admitted rebases.
- Report risk/coverage and p50/p95/p99 latency; do not optimize away abstention.
- Evidence-only overhead target: <5% median and <10% p99 on the same solved operator, explicitly prospective and not an acceptance fact.

## Claim ceiling

v0.1 pins a single contact-free one-link benchmark case and the comparison contract. It does not yet implement Drake, MuJoCo or Pinocchio adapters, run vendor binaries, qualify contact/friction, establish a production solver ranking, or claim SOTA superiority. The synthetic result fixture is a NOT_RUN schema example and must never be placed in a timing table.

## Executable kernel slice

The dependency-free Rust runner compares two implementations of the same frozen mass Riesz subproblem `M a = f` for both basis families:

- frame-typed Cholesky/Riesz raising from `metric-induced-error-envelope-reference`;
- a separate partial-pivoted LU factorization and back-substitution implementation.

It emits four machine-readable records (two algorithms × two coordinate rebases), including coordinate norm drift, tangent/cotangent metric-norm drift, dual-pairing drift, congruence defect, normwise/componentwise backward error, dual-metric residual norm, wrong-variance negative-control error, and p50/p95/p99 timings. The runner is linked to the frame-typed reference crate by a local Cargo path dependency and uses no external Rust dependencies.

Run it from the repository root:

    cargo fmt --manifest-path research/metric-benchmark-comparison-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/metric-benchmark-comparison-001/Cargo.toml
    cargo clippy --locked --manifest-path research/metric-benchmark-comparison-001/Cargo.toml --all-targets -- -D warnings
    cargo run --release --locked --manifest-path research/metric-benchmark-comparison-001/Cargo.toml --bin matched-kernel-runner -- results.json

These are measured **mass-Riesz kernel** results only. The report deliberately sets overall acceptance to `INCONCLUSIVE`: it has not executed the nonlinear constrained midpoint step or attached an independent high-precision forward-error oracle. A `COMPARABLE` status means only that the two internal algorithms solve the same pinned mass-kernel task; it does not rank them as full physics solvers.

## Next adapter gate

The next increment should add one adapter at a time, beginning with Drake's mass/inverse-dynamics path and then MuJoCo/Pinocchio. Each adapter must parse the pinned source fixture, prove the mapping fields in `benchmark-case-v0.1.schema.json`, and produce a result validated by `benchmark-result-v0.1.schema.json`. A nonmatching integrator or constraint formulation must remain `NOT_COMPARABLE` until a matched alternative is provided.

## Executable kernel slice

The dependency-free Rust runner compares two implementations of the same frozen mass Riesz subproblem `M a = f` for both basis families:

- frame-typed Cholesky/Riesz raising from `metric-induced-error-envelope-reference`;
- a separate partial-pivoted LU factorization and back-substitution implementation.

It emits four machine-readable records (two algorithms × two coordinate rebases), including coordinate norm drift, tangent/cotangent metric-norm drift, dual-pairing drift, congruence defect, normwise/componentwise backward error, dual-metric residual norm, wrong-variance negative-control error, and p50/p95/p99 timings. The runner is linked to the frame-typed reference crate by a local Cargo path dependency and uses no external Rust dependencies.

Run it from the repository root:

    cargo fmt --manifest-path research/metric-benchmark-comparison-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/metric-benchmark-comparison-001/Cargo.toml
    cargo clippy --locked --manifest-path research/metric-benchmark-comparison-001/Cargo.toml --all-targets -- -D warnings
    cargo run --release --locked --manifest-path research/metric-benchmark-comparison-001/Cargo.toml --bin matched-kernel-runner -- results.json

These are measured **mass-Riesz kernel** results only. The report deliberately sets overall acceptance to `INCONCLUSIVE`: it has not executed the nonlinear constrained midpoint step or attached an independent high-precision forward-error oracle. A `COMPARABLE` status means only that the two internal algorithms solve the same pinned mass-kernel task; it does not rank them as full physics solvers.

## Next adapter gate

The next increment should add one adapter at a time, beginning with Drake's mass/inverse-dynamics path and then MuJoCo/Pinocchio. Each adapter must parse the pinned source fixture, prove the mapping fields in `benchmark-case-v0.1.schema.json`, and produce a result validated by `benchmark-result-v0.1.schema.json`. A nonmatching integrator or constraint formulation must remain `NOT_COMPARABLE` until a matched alternative is provided.
