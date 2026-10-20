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
