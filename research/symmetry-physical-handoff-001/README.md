# Symmetry-to-physical evidence handoff v0.1

This package is the first machine-readable contract slice of [PHYS-SYMMETRY-EVIDENCE-HANDOFF-001 #4781](https://github.com/Luminous-Dynamics/symtropy/issues/4781).

## Authority separation

The source repository may report that a declared action or generator family passed mathematical checks. The receiving physics repository must independently verify that the represented physical problem and its discrete numerical step respect the claimed transformation.

The envelope therefore has two different receipts: a source mathematical receipt binding the upstream commit/checker/action, and a receiver physical receipt binding Symtropy's own commit/checker and each physical obligation. A source-side PASS never copies into the receiver-side status.

## Receiver obligations

The receiver records separate outcomes for mass-metric congruence, force covector duality, canonical momentum one-form, constraint Jacobian transformation, generator metric isometry, discrete-step equivariance, physical observables, conditioning-aware error envelope, and full task semantics.

The full-task digest binds candidate domain, objective, feasibility constraints, initial/boundary conditions, observable mapping, and answer interpretation. A symmetry of an equation alone is not sufficient.

## Hard-pruning boundary

The field scientific_search_pruning_authorized is structurally fixed to false in v0.1. Even receiver-verified physical step-map equivalence does not establish scientific search-task equivalence; pruning is a separate Symthaea quotient-compiler obligation.

## Schema and fixture

- symmetry-physical-handoff-v0.1.schema.json — Draft 2020-12 handoff envelope schema.
- symmetry-physical-artifact-manifest-v0.1.schema.json — strict 17-role digest/path manifest schema.
- fixtures/synthetic-handoff-v0.1.json — synthetic handoff shape fixture. It deliberately records source mathematical PASS alongside receiver INCONCLUSIVE, with no physical authority.
- fixtures/synthetic-artifact-manifest-v0.1.json — synthetic manifest shape fixture; its placeholder digests do not identify real files.

The schemas validate field shape, identity formats, status vocabulary, role inventory, and selected cross-field promotion rules. They do not compute the referenced SHA-256 digests or independently verify physics. The first receiver-side byte-binding implementation is [PR #5123](https://github.com/Luminous-Dynamics/symtropy/pull/5123); it verifies exact local bytes and role-to-digest binding only. Independent physical verification of all nine receiver obligations remains a separate stage.

## Next implementation

Build a Symtropy-side Rust verifier that loads exact envelope bytes, recomputes every referenced digest, resolves source artefact identities, reconstructs transformed laws, and emits a receiver-owned receipt. It must not trust upstream PASS fields as its own conclusions.
