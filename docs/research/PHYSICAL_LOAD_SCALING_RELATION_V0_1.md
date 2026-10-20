# Receipt-bound linear load-scaling relation v0.1

## Purpose

This relation adds one property-based code-verification check to the physical-solver contract. When a supported linear-static model is unchanged and all applied point-force vectors are multiplied by a positive scalar alpha, normalized response observables are expected to multiply by alpha.

This is a consistency check, not an analytical solution and not physical validation. A wrong solver can still satisfy a scaling relation.

## Strict applicability envelope

The current implementation scores only:
- model fidelity `analytical_linear_elastic` or `continuum_linear_elastic`;
- requested physics exactly `static_linear_elastic`;
- one unchanged scene digest/revision and coordinate frame;
- identical materials, boundary conditions, unit declarations, acceptance policy, required-output inventory and every other non-load request field;
- identical non-vector load metadata and IDs, with only point-force `vector_xyz` values transformed;
- a strictly positive load factor different from one;
- matching normalized-observable IDs and units for both runs.

Distributed loads, pressure, body acceleration, geometric nonlinearity, nonlinear materials, contact, damage/fracture, dynamics, thermal coupling and mixed-physics requests are unresolved/out of scope. A case outside these preconditions must not be counted as a relation pass.

## Relation and scoring

For every declared scalar observable:

```text
expected_scaled_value = baseline_value * alpha
absolute_error = abs(scaled_observed_value - expected_scaled_value)
allowed_error = absolute_tolerance + relative_tolerance * abs(expected_scaled_value)
```

The relative-error field is undefined when the expected scaled value is zero. Arithmetic overflow or unit disagreement makes the relation UNRESOLVED. An eligible, fully evidenced result outside tolerance is FAIL.

Every required output kind must have exactly one output pair in the manifest. The manifest binds both exact request-byte SHA-256 digests and freezes the positive scale factor and comparison tolerances. Both runs must have PASS receipts and PASS artifact-byte verification before scoring.

## CLI

After building the contract crate:

```sh
cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml -- \
  --request ./runs/baseline/request.json \
  --receipt ./runs/baseline/receipt.json \
  --scaled-request ./runs/scaled/request.json \
  --scaled-receipt ./runs/scaled/receipt.json \
  --artifact-root ./runs \
  --load-scaling-manifest ./benchmarks/load-scaling-relation.json
```

The artifact root must contain both run output bundles at the paths recorded by their receipts.

## Synthetic tests

Tests cover exact linear response scaling, a deliberately wrong scaled response, zero-valued observables with undefined relative error, a request whose force vector does not match the declared factor, manifest digest drift, tampered output bytes, non-load material changes and nonlinear model fidelity. All test receipts are synthetic; no external FEM solver is run by these tests.

The CI CLI fixture additionally exercises one eligible synthetic PASS and a tampered-output UNRESOLVED control. It is only evidence that the contract and CLI behave correctly on known inputs.

## Claim ceiling

A PASS indicates only that the declared, receipt-bound observables obey the load-scaling relation within the frozen tolerances under the asserted narrow linear-static envelope. It does not prove the finite-element equations are implemented correctly, the chosen model represents reality, the reference/load interpretation is independent, or the structure is safe.
