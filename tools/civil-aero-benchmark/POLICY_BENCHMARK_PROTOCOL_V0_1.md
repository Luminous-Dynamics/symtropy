# Validated Pareto quality per total budget protocol v0.1

## Goal and evidence ceiling

Compare experiment-selection and optimization policies by the quantity of **admitted, feasible Pareto quality produced per total cost**, under matched task/replicate budgets. This benchmark explicitly refuses to count a surrogate score or an unadmitted CFD result as validated design quality.

The scorecard receives a task declaration, an operation ledger, and one disposition record for every successful high-fidelity operation. Operation costs include all calls, including failed/non-converged solver runs, low-fidelity screening, tuning, orchestration and audit. A candidate enters the Pareto set only when its output SHA-256 matches a successful high-fidelity operation and the caller supplies an admission-receipt digest, feasibility status and two finite objective values.

This is not proof that a caller-provided receipt is authentic, and this scorecard does not independently recompute CFD output. The next integrity step is to bind the receipt digest to the byte-verified artifact manifest plus independent verification/validation evidence and, eventually, a signed/attested chain of custody.

## Fair comparison rules

- Compare at least two policies for every task/replicate and require at least two independent replicates per task/policy for a scorecard to be admitted.
- Policy sets and replicate sets must be complete and identical within each task.
- Budget caps must match for each task/replicate/policy.
- Objective names, directions, a lower/ideal point and the upper/reference point are fixed by task before comparisons. The task-declared box bounds make hypervolume fractions comparable across tasks without confusing raw units; their bounds must be justified and frozen before evaluation.
- This initial implementation is two-objective minimization only.
- Every successful high-fidelity operation receives exactly one disposition: admitted or not admitted.
- A failed high-fidelity operation is charged and counted as an attempt, but it cannot produce a validated candidate.
- A candidate output hash must equal the matching successful high-fidelity ledger output hash.
- An admitted candidate requires a syntactically valid 64-character admission receipt SHA-256.
- Non-admitted candidates may not carry objectives, feasibility or a purported admission receipt.
- Every feasible admitted point must lie inside the declared reference box.

## Metrics

Per task/replicate/policy, report:
- total normalized cost, remaining budget and budget utilization;
- wall seconds and CPU seconds;
- operation count, successful operations and failed operations;
- high-fidelity attempts and successful high-fidelity operations;
- admitted and not-admitted candidates;
- feasible admitted candidates;
- feasible yield = feasible admitted candidates / all high-fidelity attempts, including failed attempts;
- admission yield = admitted candidates / all high-fidelity attempts;
- 2D validated Pareto hypervolume (HV);
- normalized HV fraction = HV / ((reference_1 - ideal_1) * (reference_2 - ideal_2));
- HV per total cost and normalized HV per total cost.

Hypervolume uses a fixed task-specific reference point in minimization space. Only feasible admitted candidates contribute. Dominated points do not inflate the volume. The value is reported separately for every replicate; do not hide task variance behind a single mean.

## Synthetic known-answer

The synthetic example contains two policies, two replicates and five operations per policy:
- one low-fidelity screen;
- two admitted high-fidelity candidates;
- one successfully executed but explicitly not-admitted high-fidelity candidate;
- one failed high-fidelity attempt.

Both policies use a total budget cap of 8 units. Uniform costs 6.75 units and has two feasible admitted candidates with HV 9.5. VOI costs 5.0 units and has two feasible admitted candidates with HV 10.04. The finite fixture therefore has better HV/cost for the VOI policy while charging the failed run and excluding the not-admitted output from hypervolume.

The Rust unit tests and independent standard-library Python oracle recompute all four scorecard rows. A single-seed-only policy matrix is a required negative control and is rejected. Additional negative controls reject output digest mismatch, missing admission receipt, omitted disposition for a successful high-fidelity run, out-of-reference objective, duplicate disposition and incomplete replicate matrix.

This is arithmetic and policy-scorecard plumbing only. It is not aerodynamic data, proof of model superiority or proof that expected value of information improves physical design.

## Recommended SOTA comparison ladder

1. Fixed-seed random sampling/search.
2. Uniform or space-filling design (Sobol/LHS) with a frozen acquisition budget.
3. Bayesian optimization with declared kernels/surrogates and acquisition function.
4. Gradient/adjoint optimization where differentiability and solver derivatives are applicable.
5. Evolutionary optimization (CMA-ES/differential evolution).
6. Symthaea/Symtropy policy using the same task and cost accounting.

For ShapeBench, compare the adapter-compatible tasks under frozen, license-compliant datasets and the benchmark authors' recommended protocols. For DPW8, keep steady V1a, unsteady buffet, static deformation, high-angle and aeroelastic tasks separate. Do not transfer task rankings from one family to another.

Use multiple seeds and hold out complete geometry families and flow regimes. Charge surrogate training, policy tuning, failed simulations, mesh generation, workflow management and independent auditing. Report median and worst-decile task outcomes, not just best-case hypervolume. A stretch research target is 20–30% fewer high-fidelity evaluations to achieve equal or better validated HV and feasibility on a blind confirmation set, with no degraded worst-decile task performance; this is an aspirational target, not an achieved result.

## References

- ShapeBench: https://arxiv.org/abs/2605.20763
- SU2: https://github.com/su2code/SU2
- AIAA DPW8/AePW4: https://www.aiaa-dpw.org/TestCases/testcases.html


### Preferred CFD admission receipt binding

For a CFD-derived candidate, populate candidate.admission_receipt_sha256 with the receipt_sha256 returned by create_cfd_evidence_chain_receipt. That helper re-verifies the eight per-grid artifact roles, recomputes the numerical-admission report and emits a deterministic composite digest over the artifact bundle and numerical evidence. This scorecard still checks only the digest's syntax and does not verify the underlying receipt or its signer. A signed runner attestation remains future work.
