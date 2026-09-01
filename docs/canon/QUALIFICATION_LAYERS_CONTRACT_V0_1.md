# Qualification Layers Contract v0.1

**Status:** canonical architecture/release contract  
**Date:** 2026-09-01  
**Scope:** retained research artifacts, deterministic simulation authorities, CUF integration, evidence promotion

## Why qualification needs layers

A single word such as "qualified" can hide materially different claims.

For a retained cumulative artifact we may need to know independently:

- did these exact historical bytes apply to the expected parent?
- did the authored code compile/test/lint without modification?
- are newly discovered continuation/replay invariants satisfied?
- does a new cross-domain adapter preserve units/authority/provenance?
- does the integrated living-world scenario behave deterministically end to end?

Passing an earlier layer never implies a later one.

## Q0 — Artifact integrity and applicability

Claim:

> These are the exact retained bytes, and they apply only to the expected source lineage.

Evidence includes:

- artifact SHA-256;
- expected path count/shape;
- exact preimages for modified files;
- absence of supposed-new targets;
- `git apply --check`;
- explicit parent Git head/tree.

Q0 does not compile or validate semantics.

For Universal Matter v4.8 this is the guarded preflight layer.

## Q1 — Authored-tree build qualification

Claim:

> The exact retained authored tree builds and passes the tests/lints that existed in the authored qualification contract under the recorded toolchain/environment.

Evidence includes:

- exact Q0 artifact;
- exact staged tree;
- Rust/Cargo/Nix/platform identity;
- lockfile identity;
- format/test/clippy results;
- repository invariant gates;
- no unexpected qualification side effects;
- PASS/FAIL evidence capsule.

Q1 must not silently include repairs.

If Q1 fails, preserve the exact authored replay as explicitly **unqualified authored state** before applying fixes.

Q1 is historical/build evidence. It does not erase semantic problems discovered after the artifact was authored.

## Q2 — Continuation and replay qualification

Claim:

> The repaired deterministic authority state is sufficient for exact continuation/replay claims covered by the contract.

Q2 evaluates invariants that may not have existed in the retained artifact's original test suite.

For the current Universal Matter review this includes at minimum:

- Hydrology physical-state vs active-frontier continuation identity (#76/#79);
- Thermal physical-state vs active-frontier continuation identity (#79);
- ActiveLava physical-state vs step-sensitive continuation identity (#79);
- nested ActiveEruption checkpoint/continuation identity (#79);
- landscape environmental integration residual persistence or exact replacement (#81);
- lifecycle reconstruction of derived dirty/runtime caches where required.

Evidence must include hidden-state sensitivity and continuation-sufficiency tests, not only snapshot round trips.

Q2 may be green only on a repaired descendant of an explicitly preserved Q1 artifact lineage.

A Q1 PASS does not imply Q2 PASS.

## Q3 — Native cross-domain integration qualification

Claim:

> Qualified native authorities are exposed across domain boundaries without changing ownership, units, time semantics, or provenance.

For CUF v0.11 this includes:

- exact qualified native parent from Q2 where continuation identity is required;
- unit-explicit Matter/SurfaceWater/Ecosystem/Groundwater observations;
- typed surface/voxel spatial bindings;
- multi-source provenance for resolved groundwater;
- Matter-backed derived watershed/hydrogeology potential;
- deterministic weather as forcing evidence, not Climate authority;
- exact-time atomic read rules;
- read-only adapter boundary;
- no Basin/Terrain duplicate truth.

Q3 is adapter/integration evidence. It does not by itself prove a complete living-world scenario.

## Q4 — Causal vertical-slice qualification

Claim:

> A bounded end-to-end world scenario closes the intended physical and causal chain under deterministic replay.

Candidate Living Watershed chain:

1. deterministic weather forcing;
2. domain-owned runoff/surface-water mutation;
3. native water state/digest publication;
4. Matter-backed drainage relevance;
5. fresh downstream authority state;
6. CUF native observation;
7. LivingWatershedPolicyV2 decision;
8. Basin-owned response;
9. causal receipt closure;
10. save/reload/revisit reproduction.

Later Q4 extensions can add:

- conserved groundwater infiltration (#77);
- erosion/sediment;
- wetland emergence;
- vegetation succession;
- fire/recovery;
- settlement/infrastructure response.

Q4 must explicitly identify the bounded scenario and cannot be generalized to "the entire game is correct."

## Q5 — Scale and long-timescale qualification

Optional later claim:

> The same contracts remain stable under multi-resolution, accelerated-time, regional/planetary, and interplanetary workloads within declared error/performance bounds.

Examples:

- aggregate/detail round-trip bounds;
- one-year / ten-year / century fast-forward;
- watershed-region activation/recompression;
- catastrophe and ecological recovery;
- city/settlement lifecycle;
- orbital/stellar forcing;
- planet-scale streaming budgets.

Q5 requires explicit numerical/performance envelopes and is not implied by a small vertical slice.

## Evidence naming rule

Every status document/evidence capsule should state its layer.

Prefer language such as:

- `Q0 artifact-applicable`;
- `Q1 authored-tree PASS`;
- `Q2 continuation PASS`;
- `Q3 CUF-native-integration PASS`;
- `Q4 Living-Watershed-v1 PASS`.

Avoid bare statements such as:

- "v4.8 is qualified";
- "CUF is green";
- "the physics is verified"

unless the exact qualification layer and scope are immediately clear.

## Promotion rule

A promoted Git commit must identify the exact evidence layer(s) it satisfies.

For a pristine Q1 green path:

`Q0 retained artifact -> Q1 exact staged tree -> exact code commit`

For a repaired continuation path:

`Q1 authored replay -> focused repairs -> Q2 cumulative qualified head`

For CUF native work:

`Q2 qualified authority head -> v0.11 adapters -> Q3 qualified integration head`

For the first real watershed:

`Q3 integration head -> scenario code/evidence -> Q4 vertical-slice head`

Do not squash away the evidence-significant distinction between authored replay and repair lineage while qualification is being established.

## Red-path rule

A failure is evidence.

When a layer fails:

1. preserve the exact failing tree/evidence capsule;
2. state which layer failed;
3. do not weaken the gate to manufacture green status;
4. fix the narrow cause in child commits;
5. rerun the complete relevant layer;
6. preserve earlier failure evidence for provenance.

## Compatibility with CI tiers

Public/dependency-light CI may provide useful partial evidence without satisfying a qualification layer in full.

For example, CUF Tier A CI can prove that `symtropy-sim-contracts` builds/tests/clippy under pinned Rust without private dependencies.

That is a component signal, not Q1/Q2/Q3 for the complete private world stack.

CI labels should describe what actually ran.

## Final invariant

Qualification should answer a precise question about exact bytes, exact semantics, and exact scope.

A narrower truthful green result is more valuable than a broad ambiguous one.