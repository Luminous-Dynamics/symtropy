# Physical solver handoff schemas v0.1

Tracking: [PHYS-COUPLING-001 #2241](https://github.com/Luminous-Dynamics/symtropy/issues/2241).

- `solver-request-v0.1.schema.json`: exact scene/source revision, scene digest, model fidelity, requested physics, frame, SI units, material provenance, loads, boundary conditions and acceptance criteria.
- `solver-receipt-v0.1.schema.json`: request digest, adapter/executable/configuration identity, status, evaluated/omitted physics, convergence, output artifacts, uncertainty, validity envelope, claim ceiling and diagnostics.
- `fixtures/analytical-bar-request-v0.1.json`: synthetic request matching the analytical bar reference; values are illustrative and not measured.
- `fixtures/unsupported-dynamic-fracture-request-v0.1.json`: adversarial request that a topology + linear analytical adapter must reject or leave unresolved.
- `fixtures/unresolved-receipt-v0.1.json`: illustrative non-PASS receipt for the unsupported-physics case.

## Validation state

The JSON files were parsed for JSON syntax after being written. This is not JSON Schema conformance, cross-field semantic validation, adapter execution, or physical validation. Schema tests, an executable Rust contract validator and exact-head CI are still required before the contract is qualified.

The schemas are versioned research contracts. They must not be read as evidence that a given solver capability exists. A request for physics not in the selected adapter's explicit capability set must remain `UNRESOLVED`, not fall back silently to a weaker model.


## Executable request contract validator

The standalone Rust crate at `research/physical-solver-contract/` validates the request's exact top-level shape, accepted enumerations, lowercase revision/digest fields, SI unit declaration, duplicate identifiers, material ranges, loads, boundary DOFs and acceptance tolerances. It computes SHA-256 over the exact request bytes and emits a versioned request-validation receipt.

Important status distinction:

- `request_validation = PASS` means only that the request passed this validator's structural and cross-field checks.
- `physical_result_status = UNRESOLVED` is always emitted because the crate does not invoke a physics solver.
- Unsupported or absent physics is listed in `physics_omitted` and prevents the capability screen from claiming a supported request envelope.

The validator's receipt schema is `solver-validation-receipt-v0.1.schema.json`. Focused CI runs pinned Rust formatting, tests, Clippy, exact-head receipt assertions and negative controls. This remains a request-contract validator, not a complete JSON Schema implementation or a qualified solver adapter.
