# Receipt-bound load-scaling relation v0.1

**Implementation:** `research/physical-solver-contract/src/load_scaling.rs`.  
**CLI:** `--load-scaling-manifest` with baseline/scaled request and receipt pairs.  
**Research parent:** [PHYS-META-001 #2409](https://github.com/Luminous-Dynamics/symtropy/issues/2409).  
**Claim ceiling:** metamorphic numerical consistency only; no physical validation or solver qualification.

## Why this relation matters

For small-strain linear elasticity with fixed geometry, material/operator, boundary conditions, mesh, and solver configuration, scaling all applied point forces by positive `alpha` should scale linear response quantities (e.g., displacement, strain, stress and reaction resultants) by `alpha`. This checks a relation across two runs, complementing analytical-reference checks.

It is not universal: contact state, plasticity, geometric nonlinearity, follower loads, damage, load-dependent boundary conditions or a changed solve configuration can invalidate this property. The assessor accepts only requests that declare `analytical_linear_elastic` or `continuum_linear_elastic` fidelity and `static_linear_elastic` as the sole requested physics, with point-force loads. All non-load request semantics must match; only `request_id` and point-force vectors may differ.

## Relation manifest

The manifest binds the exact baseline and scaled request SHA-256 values. It names the case, scale factor, shared absolute and relative tolerances and exact output pairs. Every required output kind must be covered; each selected output must be unique, receipt-declared, byte-verified and normalized with exact request/scene identity.

The v0.1 contract binds allowed output kinds to exponents:
- `displacement`, `stress`, `reaction_force`, `reaction_moment` require exponent `1`;
- `strain_energy` requires exponent `2) and an SI joule unit (`J`);
- ambiguous or other output kinds, and any kind/exponent mismatch, are rejected as unsupported.

This mapping is enforced by both Rust and the JSON schema instead of trusting a caller-selected exponent. The normalized values still need correct semantic meaning; a dishonest or incorrect producer can emit mislabeled values that satisfy a relation. That is why this check complements, rather than replaces, independent analytical references and load/reaction equilibrium.

For a baseline observable (q) and load factor (alpha>0), the checked relation is:

```text
q_scaled ~= q_baseline * alpha^p
```

where (p) is the explicit per-output exponent. The acceptance criterion is `absolute_error <= absolute_tolerance + relative_tolerance * abs(expected)`. If the expected value is zero, relative error is undefined rather than fabricated as zero. Non-finite expectations or errors are `UNRESOLVED`; valid finite deviations outside tolerance are `FAIL`.

## Fail-closed evidence

- Baseline and scaled requests must have distinct request IDs and bind the same scene revision, coordinate frame, physics regime and all non-load inputs.
- The relation manifest must bind exact canonical request bytes.
- Every scaled load vector must equal the declared positive factor times its baseline vector component by component.
- Every required output kind must have exactly one declared output pair.
- Both solver receipts and every output's bytes must verify before comparison.
- Each normalized output must bind exact request SHA, request ID, scene SHA/revision and case ID.
- Observable inventories, units and receipt-declared output units must match.
- Missing, malformed, stale or unsupported artifacts, changed model inputs and unit mismatches are `UNRESOLVED`, not a numerical PASS.
- Reports keep `physical_result_qualified=false` and `real_world_validation_status=UNRESOLVED`.

## Initial synthetic tests

Tests use hand-authored normalized JSON and synthetic receipts; no external FEM solver executes in the test fixture.

1. Exact linear response scaling -> PASS.
2. Response scaled by an extra 10% -> FAIL.
3. Exact zero response -> relative error undefined.
4. Request load not matching the declared factor -> UNRESOLVED.
5. Manifest/request digest drift -> UNRESOLVED.
6. Tampered normalized artifact -> UNRESOLVED.
7. Modified Young's modulus between runs -> UNRESOLVED.
8. Nonlinear fidelity -> UNRESOLVED.
9. Factor one (no transformation) -> rejected as a meaningless scaling case.
10. Explicit quadratic exponent two -> verifies scaling by (alpha^2).
11. Unsupported exponent (e.g. three) -> rejected.

These controls test the assessor's decisions, not FEM accuracy or physical validation.

## CLI

```sh
cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml -- \
  --request baseline/request.json \
  --receipt baseline/receipt.json \
  --scaled-request scaled/request.json \
  --scaled-receipt scaled/receipt.json \
  --artifact-root ./artifacts \
  --load-scaling-manifest ./relation.json
```

The output is machine-readable JSON. A mathematical relation PASS does not prove that an independently trusted executable produced the artifacts or that the model represents a real system.

## Extensions not yet claimed

- Vector/tensor-valued normalized observables and coordinate-transform relations.
- Superposition with three receipt-bound runs and unchanged operator/boundary data.
- Generated strata, blind holdouts and surviving-mutant reports.
- NAFEMS/MMS/independent analytical case coverage and actual solver execution.
- Measurement-based physical validation.

## Claim ceiling

The evaluator checks a declared load-scaling relation under a narrow linear-static request envelope. It does not prove solver correctness, material/geometry fidelity, mesh adequacy, physical realism, engineering safety or any performance advantage over existing FEM systems.