# Receipt-bound linear superposition relation v0.1

## Purpose

This relation checks response(A + B) approximately equals response(A) + response(B) for a tightly defined small-strain static linear-elastic request envelope. It complements the load-scaling relation; it is neither an exact analytical reference nor physical validation.

## Preconditions

All three exact requests must declare either analytical_linear_elastic or continuum_linear_elastic fidelity and static_linear_elastic as the only requested physics. They must share the exact same scene digest/revision, coordinate frame, source revision, material declarations, boundary conditions, unit declarations, acceptance settings and output inventory.

The A and B request load lists must both be non-empty. Version 0.1 accepts only point_force loads. Load IDs must be unique and disjoint between A and B. The combined request must contain every A and B load exactly once, with identical per-load metadata and force vector. Additional or missing combined loads, changed non-load fields or duplicate IDs make the relation UNRESOLVED.

The A, B and combined requests require distinct request IDs. A frozen relation manifest binds SHA-256 of all three exact request byte streams and the selected output IDs. Every required output kind must be paired. All three receipts must PASS and all receipt-declared output artifacts must pass byte verification. Selected outputs are read again and re-hashed before parsing.

## Scoring

For each scalar normalized observable:

```text
expected_combined_value = response_a + response_b
absolute_error = abs(response_combined - expected_combined_value)
allowed_error = absolute_tolerance + relative_tolerance * abs(expected_combined_value)
```

Observable IDs and units must agree across all three runs; normalized units must also agree with each output's receipt-declared unit. Unit disagreements, arithmetic overflow or evidence/identity gaps yield UNRESOLVED. Only a fully eligible, fully evidenced numerical discrepancy outside tolerance yields FAIL. Relative error is explicitly undefined when the expected combined value is zero.

## CLI

```sh
cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml -- \
  --request-a ./runs/a/request.json \
  --receipt-a ./runs/a/receipt.json \
  --request-b ./runs/b/request.json \
  --receipt-b ./runs/b/receipt.json \
  --request-combined ./runs/combined/request.json \
  --receipt-combined ./runs/combined/receipt.json \
  --artifact-root ./runs \
  --superposition-manifest ./benchmarks/superposition.json
```

The artifact root must contain all three run outputs at the relative paths in their receipts.

## Synthetic test oracle

The fixture uses independent responses (0.01, 0.00) m and (0.00, 0.02) m; the combined response should be (0.01, 0.02) m. Tests include a deliberately wrong combined response, changed load inventory, duplicate load IDs, material drift, nonlinear fidelity, request-digest drift, unit mismatch and output byte tampering. Synthetic receipt/observation fixtures do not invoke an external FEM solver.

## Claim ceiling

A PASS demonstrates only relation consistency for receipt-bound scalar observables under the defined linear-static contract. It does not prove the solver implementation is correct; multiple errors may preserve superposition, and the relation says nothing about nonlinear/contact/fracture/dynamic behavior or model-to-reality fidelity.
