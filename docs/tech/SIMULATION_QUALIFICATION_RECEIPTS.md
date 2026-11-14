# Simulation Qualification Receipts

**Status:** protocol design, not yet a serializer or verifier implementation.  
**Tracking:** [SIM-QUAL-001 #2908](https://github.com/Luminous-Dynamics/symtropy/issues/2908).

## Principle

Every result used to support a scientific or engineering claim must answer two separate questions:

1. **What exactly ran?** Exact source, input, configuration, toolchain, command, output artifacts, and reference oracle.
2. **What did the evidence justify?** A bounded disposition that distinguishes verified results, detected failures, unresolved cases, skipped checks, and missing evidence.

A green process exit is execution evidence. It is not, by itself, a correctness proof, an uncertainty analysis, or evidence that a model applies outside the tested case.

This receipt format is intended to unify the current periodic geometry, periodic collision, region-interface, native SPH, external CFD and Symthaea experiment-selection programmes. It does not replace their domain-specific equations or acceptance gates.

## Receipt layers

### 1. Identity and provenance

Required immutable identity fields:

- schema version, benchmark ID, benchmark revision, scenario ID, and scenario revision;
- repository identity, exact source commit SHA, working-tree cleanliness, Cargo.lock digest and enabled feature/configuration set;
- toolchain/compiler version, relevant compiler flags and target triple;
- input, geometry/mesh, configuration and initial-state digests;
- declared units, coordinate frame, physical/topological model and validity envelope;
- PR/CI run identifiers when execution was hosted in CI.

A missing source SHA or missing configuration digest makes the receipt invalid for a reproducibility claim.

### 2. Execution outcome

Record the invoked command as an argument vector, exit code, timeout/resource outcome, raw stdout/stderr artifact digests, run duration, and whether outputs were fully produced. Do not store an unbounded log inline; reference content-addressed artifacts.

Execution status answers whether a process ran. It does not decide scientific validity.

### 3. Oracle and verdict

Each claim names its oracle or verifier, version/digest, test method, tolerance/metric schema, and independence relationship to the implementation. The receipt must state whether the oracle shares code with the production path; independence cannot be assumed from a type name.

Use a closed disposition enum:

- **Pass:** all mandatory checks for the named claim executed and passed on the declared corpus.
- **Fail:** at least one mandatory check produced a definitive violation.
- **Indeterminate:** evidence was insufficient to determine the result, an oracle returned unresolved, or a numerical/budget limit was reached.
- **NotRun:** execution never began or a required result is absent.
- **Skipped:** the check was explicitly not selected by the declared protocol.
- **InvalidReceipt:** required metadata, digests, artifacts or invariants are absent or inconsistent.

Do not implicitly convert Skipped, NotRun, timeout, missing output or Indeterminate into Pass. Budget exhaustion must not become separation, and missing numeric results must not become zero.

### 4. Metrics and scientific claim ceiling

Each metric records its name, schema version, unit, normalization, tolerance, value or typed failure reason, and whether it is a hard gate or a performance score.

Hard correctness gates and performance metrics are separate channels. A benchmark may report p50/p95/p99 latency, throughput, memory, candidate counts, convergence cost or sample efficiency only after the required correctness gate passes. A weighted score must never hide a hard-gate violation.

Every receipt states its validity envelope: dimension, shape family, topology/periodic mask, numeric ranges, timestep, model equations, boundary conditions, reference regime, and explicit exclusions. Passing one bounded corpus is not a general proof of an entire solver.

## Determinism and digest rules

Pin a schema and canonical serialization version before implementing hashes. The canonical scientific payload should include immutable source/scenario identity, oracle identity, disposition, metric values, failure reasons and referenced artifact digests.

Separate volatile operational metadata—timestamps, worker names, transient CI URLs and host labels—from the deterministic result digest. Changing only volatile metadata should not change the deterministic scientific-result digest. Hardware/runtime details remain in the full receipt when relevant to interpreting timings.

A content digest establishes that bytes match a declared digest. It does not establish the truth of a claim. An optional signature can establish who emitted a receipt, but signatures do not make a biased or dependent oracle scientifically valid.

## Append-only correction model

Receipts are append-only evidence. A rerun or corrected result gets a new receipt and may reference a prior receipt as superseded. Do not erase the previous failure or reuse its identifier for new output. The failure fixture and its minimized input should remain attached to the benchmark revision until the correction is verified.

## Domain-specific acceptance examples

### Periodic geometry and sphere CCD

Required payload:
- domain origin, per-axis period and periodic-axis mask;
- fixed PRNG algorithm/seed and corpus revision;
- image-candidate budget and per-image TOI output;
- independent brute-force oracle identity and test construction;
- candidate recall, conservative extra-candidate count, TOI error, unresolved count and deterministic ordering;
- unwrapped trajectory digest, event order, momentum/energy residual and event-budget status for the reference world.

Hard gates include no omitted overlapping image, no false definitive separation on the frozen oracle corpus, non-finite-state rejection, deterministic image ordering, and transactionality on event-budget failure. Broadphase extras are overhead, not collision truth.

### Region interfaces

Required payload:
- source/target region IDs and model revisions;
- exact interface ID/revision, affine-map and local-clock parameters;
- expected body/interface/region revision preconditions;
- declared momentum/energy policy, exchanges and residuals;
- before/after state identity, transition receipt and replay result.

A transition is not Pass merely because the map returned a vector. Boundary admission, velocity/direction consistency, conservation or explicit-transform policy, exact-once authority, stale revision handling and idempotent replay must all satisfy the declared contract.

### Native SPH

Required payload:
- kernel, normalization, EOS, viscosity law, unit conventions and regularization parameters;
- particle initial condition, support radius, wall/ghost treatment, timestep and solver profile;
- reference case and reference-data digest;
- spatial resolution, temporal resolution, L1/L2/L-infinity error, convergence-rate fit, mass/momentum/energy diagnostics, neighbor count, pressure/density extrema and raw field-output digest.

Do not call a solver validated based on visually smooth trajectories or on one successful timestep. Unconverged/non-finite cases stay in the corpus and budget.

### External CFD

Required payload:
- executable/version/build digest, mesh/config/input digests, model and boundary-condition settings;
- convergence histories, conservation/force-balance diagnostics and output digests;
- named reference data, uncertainty and matched coefficient definitions;
- resource usage, solver status and whether the case is inside the reference regime.

Successful process exit without convergence evidence is not a scientific pass.

### Symthaea experiment selection

Required payload:
- identical initial design, method/acquisition implementation and version;
- all high-fidelity call outcomes, including failures and retries;
- objective/constraint definition, compute budget, validation/confirmation split and oracle data digest;
- achieved feasible objective/regret, uncertainty calibration, out-of-domain abstention and resource cost.

Failed/unconverged simulations count against the budget. Confirmation data may not be used for tuning.

## Independent negative controls

The receipt/verifier suite must prove it can detect:
- changed raw-output bytes with an unchanged claimed digest;
- wrong/missing scenario or lockfile digest;
- a missing required oracle artifact;
- a missing periodic image, deliberately corrupted TOI, or non-deterministic result order;
- non-finite output or event/image budget exhaustion mislabeled as success;
- a transition replayed with conflicting data;
- an SPH result lacking one required refinement level;
- a CFD output marked valid despite failed convergence;
- a Symthaea method given extra hidden oracle information or omitted failed trials.

A negative control is required evidence that the verifier can fail—not decorative test coverage.

## Implementation status

The first typed Rust implementation is in draft [PR #2910](https://github.com/Luminous-Dynamics/symtropy/pull/2910), package `symtropy-qualification`. Its CI has been started but the current exact-head results must be checked before adopting the API. The implementation currently provides:

- typed provenance, scenario, oracle, execution, case-count, metric and artifact records;
- six dispositions with fail-closed constraints;
- payload SHA-256 digest and artifact-byte/size verification;
- canonical ordering for map-like metadata, artifacts, metrics, feature flags and validity-envelope labels;
- rejection of non-independent oracle passes, dirty-source passes, missing required artifacts, incomplete case accounting, non-finite metrics and integer overflow;
- negative-control unit tests for changed bytes, tampered payload, NotRun/Skipped/timeout, missing artifacts, duplicate metrics and volatile-metadata stability.

The digest binds the declared payload, not the truth of its claims. An independence field is a declaration, not proof that the oracle is independent. Referenced artifact bytes still need to be retrieved and verified by the caller.

## Migration plan

1. Verify the exact-head unit tests and Clippy for [PR #2910](https://github.com/Luminous-Dynamics/symtropy/pull/2910), then freeze the schema/serialization contract.
2. Add golden serialization fixtures and test compatible/unsupported schema versions before publishing receipts.
3. Emit only truthful NotRun/Indeterminate receipts where qualification has not executed; never fabricate successful data.
4. Convert the periodic differential corpus and WCSPH benchmark to receipts while retaining the raw logs and exact-head workflow IDs.
5. Add the SU2 runner and adaptive-search benchmark only after the common receipt/verifier passes its own negative controls.

## Claim ceiling

A valid receipt makes a bounded execution and verifier verdict independently inspectable. It does not itself prove physical truth, oracle independence, model adequacy, generalization, or SOTA superiority.
