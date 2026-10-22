# Landscape Scenario Contract v0

**Status:** contract and validation gate; not a qualified environmental simulator.  
**Owner:** Symtropy Basin (crates/domains/symtropy-basin).  
**Upstream project artifact:** [Sol-Atlas Landscape Design Workbench](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/191).  
**Cross-repository workstream:** [LAND-SIM-001 #2298](https://github.com/Luminous-Dynamics/symtropy/issues/2298).

## Paradigm shift: do not let a visualization become a model input by implication

A visual design document is an authored proposal. A simulation input is a model contract with a declared frame, units, resolution, boundary conditions and a supported domain. The first cannot become the second simply because its JSON parses or its layer looks realistic.

The Sol-Atlas v0 editor currently exports LOCAL_CARTESIAN_DRAWING_UNITS with georeferenced=false. Its objects use schematic-symbol-anchor-v0, not full surveyed geometry. Therefore the contract permits:

- schema/contract qualification of the exact document bytes;
- synthetic benchmark execution where document and feature provenance are explicitly synthetic.

It blocks analytical-reference and scientific-simulation claims from the v0 local drawing frame. Passing this contract is not evidence that a hydrology, shade, biodiversity, soil, erosion or engineering model is valid.

## Rust API

The module symtropy_basin::landscape_scenarios defines:

- DesignSnapshotRef: schema, schema URI, declared scope, geometry status, frame kind, georeference declaration, feature count and SHA-256 of the exact snapshot byte sequence;
- LandscapeScenarioManifest: stable scenario ID, pinned design reference, model ID/version, purpose, seed and SHA-256 for exact boundary-condition bytes;
- ContractViolation: stable machine code, JSON path and human-readable detail;
- design_snapshot_ref(bytes): parse the minimum metadata needed for the handoff reference and hash the exact consumed bytes;
- validate_against_inputs(snapshot_bytes, boundary_conditions_bytes): compare declared references/digests to the exact inputs and apply the v0 purpose/claim gate.

The caller must validate the full Sol-Atlas source snapshot against the authoritative Sol-Atlas JSON Schema before handoff. The Symtropy gate independently rechecks claim eligibility and byte identities. It is not a substitute for that full source-document validator.

## Manifest schema

The machine-readable contract is [schemas/landscape-scenario-manifest-v0.schema.json](../schemas/landscape-scenario-manifest-v0.schema.json).

The accepted v0 manifest purposes are deliberately restricted to schema-qualification and synthetic-benchmark. The Rust enum also represents rejected analytical/scientific request classes so callers and tests can observe a stable, typed failure instead of accidentally granting authority.

Every execution record should persist the manifest bytes, snapshot bytes or durable content reference, boundary-condition bytes or durable content reference, their digests, model implementation/version, seed, toolchain/build identifier and raw outputs. A digest is an integrity reference, not a source of truth by itself; the referenced bytes and source claims must remain reviewable.

## Required test cases

The Rust unit tests include:
- SHA-256 known-answer test;
- schema/Rust-contract parity;
- deterministic manifest serialization and round trip;
- acceptance of a pinned local schematic for schema qualification;
- accepted synthetic benchmark with seed/provenance status;
- rejection when snapshot bytes differ from the declared digest;
- rejection for missing/malformed digest and boundary-condition mismatch;
- rejection of analytical/scientific purposes for the v0 drawing frame;
- rejection of synthetic input as an independent analytical reference.

## Acceptance gates

1. Same manifest and exact input bytes yield the same validation result.
2. A change of one byte in the snapshot or boundary-condition payload is detected.
3. Unknown schema/scope/purpose, unsupported frame, missing fields or excessive feature count fail closed.
4. A synthetic scene cannot be silently relabelled as a user-authored or measured design.
5. No ecological, hydraulic or construction outcome is emitted by this contract module.
6. cargo fmt --all --check and cargo test -p symtropy-basin landscape_scenarios pass on the exact PR head before merge.

## Evaluation-plan linkage

A scenario may optionally carry an `evaluation_plan` reference. When present, its SHA-256 commits to the exact Sol-Atlas evaluation-plan bytes and it names the plan schema URI and evaluation ID. Validation requires those exact bytes; it verifies the digest, plan schema identity, evaluation ID, and that the plan's `project_snapshot_sha256` and `project_scope` match the scenario's exact design snapshot reference.

- Sol-Atlas evaluation plan and strict validator: https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/210
- Evaluation-plan JSON Schema: https://github.com/Luminous-Dynamics/sol-atlas-leptos/blob/feat/landscape-evaluation-protocol-v0/schemas/landscape-evaluation-plan-v0.schema.json

The plan must first pass the authoritative Sol-Atlas validation path. This Symtropy check does not duplicate the full validator, prove consent, or establish scientific adequacy or causal identification. It merely prevents a scenario run from becoming detached from the study protocol or silently switching project versions. If no evaluation plan is pinned, legacy schema-qualification/synthetic scenarios remain valid without plan bytes.

## Next stages

- **Schema qualification:** full input validation and export reproducibility.
- **Synthetic scenario bench:** deterministic performance / algorithm testing only; results remain synthetic.
- **Measured/georeferenced input profile:** only after Sol-Atlas supplies true geometry, verified coordinate transforms, source attribution, units, spatial resolution, temporal coverage and validation evidence.
- **Domain-specific model profiles:** a water-flow model, canopy/shade model or habitat model must each have independently computed reference cases, calibrated/validated input ranges, sensitivity reporting and fail-closed out-of-domain conditions.

There is no universal landscape sustainability score in this contract. Different objectives (water, heat, habitat, access, costs, stewardship) remain separately inspectable and must disclose their trade-offs.
