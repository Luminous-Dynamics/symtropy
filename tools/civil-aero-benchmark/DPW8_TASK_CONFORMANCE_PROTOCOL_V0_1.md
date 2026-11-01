# DPW-8 task-protocol conformance audit v0.1

## Purpose and evidence ceiling

A submitted result should not be called a reproduction of a benchmark merely because its headline operating point is similar. Before numerical comparison, compare the observed case metadata against the published task definition and classify each requirement independently.

The task_conformance module distinguishes:
- conforms: observed value matches the declared protocol constraint;
- deviates: an observed value differs from a mandatory exact/range requirement, or a count is below its required minimum;
- meets_minimum_below_desirable: the required minimum is met but the workshop's desirable target is not;
- unknown: the observation is absent; absence is not zero and is not a pass.

The aggregate report preserves deviations and unknowns. It never claims physical equivalence between different boundary conditions, infers missing metadata, or repairs participant records. This is a metadata conformance diagnostic. It does not authenticate the source, reproduce the solver, assess numerical convergence, validate a physical model, or establish airworthiness.

## DPW-8 V1a protocol comparison

The official V1a ONERA OAT15A task defines:
- Mach 0.73;
- chord Reynolds number 3,000,000;
- static temperature 271 K;
- angles of attack 1.36°, 1.50°, 2.50°, 3.00° and 3.10°;
- a six-member grid family, with four levels required and six desirable;
- periodic sidewall boundary conditions.

Official task definition: https://www.aiaa-dpw.org/TestCases/testcases.html

The pinned IISc participant submission metadata identifies:
- SU2 8.0.1, RANS, SA Edwards–Chandra;
- Mach 0.73, chord Reynolds number 3,000,000, static temperature 271 K;
- the five V1a angles;
- four grid levels (47,187; 89,616; 150,333; and 235,491 nodes);
- a hybrid grid and a symmetric boundary condition.

Immutable source reference:
https://github.com/Drag-Prediction-Workshop/DPW8-Buffet/blob/4d7582633a2bed4f4fe1fe933af9a9b6f3e2ae43/TestCase1a/014_IISc/01_SU2_Symmetric_BC/DPW8-AePW4_ForceMoment_v5.dat

This yields the following metadata-only findings:
- The stated flow conditions and prescribed angles match the V1a task.
- Four grid levels meet the required minimum. Six remain desirable; this is not classified as a mandatory deviation.
- symmetric differs from the task definition's periodic sidewall condition. The audit records a deviation and does not assume the two conditions are physically equivalent.
- Exact geometry, mesh, executable, and solver-configuration byte identities needed for historical reproduction are not present in this metadata observation set. That requirement remains unknown, not passed.

No participant source bytes are copied into this repository. The submission's terms status remains pending; the immutable source URL is a provenance pointer, not a redistribution permission.

## Implementation contract

src/task_conformance.rs provides reusable typed constraints:
- exact text (optionally case-insensitive);
- inclusive numeric range with explicit absolute tolerance and unit;
- required minimum plus desirable count.

Each observation is typed, source-referenced and either present or unknown. Duplicate or unlisted observations, missing lineage for a supplied value, non-finite numbers, invalid thresholds and type mismatches are rejected. Missing observations remain unknown. Numeric values are never coerced to text, counts, or zero.

The deterministic DPW-8 demo is driven only by the versioned protocol conditions and the metadata recorded in the source-discovery manifest. It does not parse or retain the upstream participant file. A separate stdlib Python oracle checks the report's statuses, identities, source references and evidence ceiling.

## Acceptance criteria

1. Four operating-point checks and the prescribed alpha set conform.
2. The sidewall condition is reported as deviates (symmetric versus periodic).
3. Four grids are reported as meets_minimum_below_desirable (minimum four, desirable six).
4. Missing geometry/mesh/executable/configuration byte identity is unknown.
5. The aggregate result is deviations_detected_and_incomplete.
6. No result is called physically equivalent, solver-reproduced, experimentally validated, or flight-authorized.
7. Repeated runs produce byte-identical CSV, and an independent implementation checks the known-answer output.

## Research consequence

Numerical comparisons should be stratified by protocol-conformance class. A participant-submitted result with a documented boundary-condition deviation may still be useful evidence, but it must not be pooled into an exact-protocol baseline without an independently justified equivalence argument. Historical reproduction should first reproduce the participant's actual setup; a separate lane should evaluate the official task's prescribed setup. These are different questions and must have different run IDs, receipts, and comparison labels.

This separation prevents a solver-version comparison from accidentally measuring a change in both solver and boundary conditions. It also prevents a leaderboard from rewarding a result that solves an easier or different problem while presenting it as an apples-to-apples comparison.

## Next evidence required

- Ask the participant/workshop maintainers to clarify the symmetric-versus-periodic setup and whether the submitted run is a permitted task variant.
- Obtain exact geometry and grid artifacts, source-owner terms, and the complete solver configuration before attempting historical reproduction.
- Keep the historical participant setup and the strict protocol setup in separate lanes until the discrepancy is adjudicated.
