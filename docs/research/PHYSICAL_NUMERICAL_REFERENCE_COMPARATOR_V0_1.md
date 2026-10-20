# Physical numerical reference comparator v0.1

**Tracking:** [PHYS-SOLVER-QUAL-001 #2303](https://github.com/Luminous-Dynamics/symtropy/issues/2303)  
**Implementation:** draft PR for the canonical solver-contract crate; composes with output-byte verifier [PR #2307](https://github.com/Luminous-Dynamics/symtropy/pull/2307)  
**Status:** synthetic test corpus only; no external solver is invoked.

## Paradigm shift

A receipt with valid hashes is not a numerically verified result. The next verification stage must compare a selected, receipt-bound normalized output against a reference manifest whose expected values, units, oracle identity and tolerances are explicit.

The chain is:

```text
request/receipt consistency
  -> output bytes match receipt digests
  -> normalized observables match exact request-bound reference
  -> independent solver verification / convergence study
  -> experimental validation and uncertainty assessment
  -> intended-use model qualification
```

Every arrow is a distinct evidence gate. This comparator does not promote physical validation or engineering-use qualification.

## Contracts

- `physical-reference-observables-v0.1` identifies a case, exact request SHA-256, reference protocol, oracle method/identity/source, and a non-empty inventory of expected observables with units and non-negative absolute/relative tolerances.
- `physical-normalized-observables-v0.1` carries the same request SHA-256 and case ID with normalized scalar values and explicit units.
- The normalized result must be an output named by the solver receipt. All receipt outputs must first pass local exact-byte verification under the artifact root.
- The selected file is hashed again when read for comparison; if it differs from the verified digest, comparison aborts as `UNRESOLVED`.
- Unknown or duplicate metric IDs, an incomplete/extra metric inventory, units mismatch, request/case mismatch, non-finite values, malformed tolerances or invalid artifact bytes cannot produce a comparison PASS.
- Producer `FAIL` or `UNRESOLVED` is never numerically promoted to PASS.

The comparison rule for observable (i) is:

```text
absolute_error_i = |observed_i - reference_i|
allowed_error_i = abs_tol_i + rel_tol_i * |reference_i|
PASS_i iff absolute_error_i <= allowed_error_i
```

All units must match exactly. No implicit conversion occurs in v0.1. Relative error is reported only when the reference is non-zero; at zero reference, the absolute tolerance controls acceptance and relative error is explicitly undefined, not silently zero. Tolerances must be chosen before the candidate run; a digest records the manifest bytes but does not itself prove that its timing was preregistered.

## CLI

```sh
cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml -- \
  --request request.json \
  --receipt solver-receipt.json \
  --artifact-root ./output-bundle \
  --reference reference-observables.json \
  --observation-output-id normalized-observables
```

This is a narrow normalized-scalar contract. It does not yet parse solver-native mesh formats such as MED, VTK, Exodus or CSV grids. A backend-specific normalizer must define how raw solver fields map to comparable observables and retain its own version/provenance.

## Hostile / regression controls

- exact match within predeclared tolerance;
- numerical error outside tolerance -> `FAIL`;
- zero reference -> explicit undefined relative error and absolute-only decision;
- zero reference outside absolute tolerance -> `FAIL`, regardless of large relative tolerance;
- unit mismatch -> `UNRESOLVED`, with no automatic unit conversion;
- stale exact request digest in reference or normalized observations -> rejected;
- receipt output tampering / missing artifact -> no comparison PASS;
- non-PASS producer receipt -> comparison remains `UNRESOLVED`;
- unknown, missing or duplicate observable IDs -> no comparison PASS;
- reference manifest with negative, non-finite or overflowing tolerances -> fail closed.

## Benchmarks and next external-solver step

- 100% correct disposition across the frozen exact-match and hostile-control corpus.
- Independent implementation of the reference values, separate from the solver/normalizer path under test.
- For FEM adapter qualification, use an analytical cantilever/beam case, compare displacement, stress/resultant and reactions, and show at least three mesh-refinement levels. Report output-specific error, equilibrium/reaction balance, convergence direction, runtime and peak memory separately.
- Do not generalize a scalar-comparison PASS to the entire mesh field or other physics.
- Keep measured specimen validation separate. The [PHYS-SOLVER-QUAL-001 plan](https://github.com/Luminous-Dynamics/symtropy/issues/2303) identifies Code_Aster as the leading initial candidate and requires review of each case's data rights and supported scope.

## Claim ceiling

`comparison_status = PASS` means only that this exact normalized-observable artifact passed the declared reference comparison for this request and these tolerances. It does not authenticate solver execution, guarantee independent authorship, establish mesh convergence, prove the reference is correct, establish model-to-reality validity, or qualify structural safety.
