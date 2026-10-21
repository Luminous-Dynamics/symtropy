# CalculiX RR reaction-output parser v0.1

**Implementation:** `research/physical-solver-contract/src/calculix_rr.rs`  
**Qualification issue:** [PHYS-CCX-OUTPUT-001 #2468](https://github.com/Luminous-Dynamics/symtropy/issues/2468)  
**Backend gate:** [PHYS-EXTQUAL-001 #2321](https://github.com/Luminous-Dynamics/symtropy/issues/2321)

## Purpose and key hazard

This module parses one reaction-only `RR` section from a CalculiX `.dat` output, but only when the paired input deck explicitly requests `RR` for the expected node set. It deliberately does not accept ordinary `RF`/force sections as a substitute.

CalculiX documentation says `RF` is the total external force contribution at a node, combining the reaction force with concentrated `*CLOAD` and distributed `*DLOAD` contributions; it is not guaranteed to represent pure support reaction. This has caused real interpretation differences, especially when point loads act at constrained nodes. See the [`*NODE PRINT` documentation](https://www.feacluster.com/CalculiX/ccx_2.18/doc/ccx/node307.html) and [RF output explanation](https://www.feacluster.com/CalculiX/ccx_2.18/doc/ccx/node201.html).

The candidate `RR` implementation is **not an upstream capability we may assume**. At the latest repository inspection on 2026-10-21, [CalculiX upstream PR #154](https://github.com/Dhondtguido/CalculiX/pull/154), “Output only reaction forces,” remains open and draft. Its description says it assumes [PR #123](https://github.com/Dhondtguido/CalculiX/pull/123), “Acceleration output for dynamic calculations,” is merged; PR #123 is closed without merging. The RR patch author also records an FRD limitation for expanded elements. The related [March 2026 forum thread](https://calculix.discourse.group/t/output-reaction-forces-only/3809) documents further testing, including early output-path/element cases where adding RR affected other results or generated incorrect FRD values.

Therefore this parser is only a candidate adapter component. No stock CCX version is presumed to support RR. A patched build must be pinned by source commit and executable digest, the dependency/build issue resolved, and real-output regression cases passed before the parser can be used in a backend qualification study. The parser still requires the actual output section and the exact deck to request RR; it never infers capability from a version string.

## Supported parser envelope

- One `*NODE PRINT,NSET=...` card for one explicitly named support node set.
- That card must request `RR`; `RF` may also be requested, but is ignored as a reaction source.
- One `reaction forces (rfx,rfy,rfz) for set ... and time ...` section in the result bytes.
- The section's set name must match the expected support set.
- Exactly one finite three-component row for every expected support node ID and no unexpected nodes.
- Finite decimal/scientific numeric fields; `D`/`d` exponent notation is normalized for parsing.
- UTF-8/ASCII text, bounded deck/output sizes, deterministic report JSON.

Unsupported: multiple RR sections/steps, missing or ambiguous section, multi-set outputs, transformed-coordinate interpretation, reaction couples, dynamic/nonlinear/contact cases, and any raw-output grammar not explicitly supported. These cases must be handled by a later, separately tested version or abstain.

## Evidence outputs

`parse_reaction_only_rr_bytes(deck_bytes, dat_bytes, expected_nset, expected_node_ids)` reports:
- SHA-256 of the exact deck and `.dat` bytes;
- selected set name, time header and exact expected node inventory;
- each node ID, the three parsed force components as written, source line and raw source row;
- explicit warnings that unit system and coordinate frame are not inferred by the parser;
- `physical_result_qualified=false` and `real_world_validation_status=UNRESOLVED`.

`normalize_reaction_only_rr_bytes(request_bytes, deck_bytes, dat_bytes, manifest_bytes, node_catalog_bytes)` is the next pure-Rust bridge. It joins parsed RR force vectors to a separately supplied support-node catalogue only when the catalogue and manifest bind the exact request, scene revision, frame, deck digest, node-catalogue digest and expected support-node inventory. Version 0.1 accepts only the same named frame without transforms and explicit SI request units (m, N); unsupported or mismatched metadata returns an error rather than a normalized value.

The normalizer emits:
- a payload conforming to `physical-equilibrium-reaction-forces-v0.1`;
- the normalized payload SHA-256;
- exact request, deck, `.dat`, manifest and node-catalogue digests;
- an explicit statement of unit and frame provenance;
- `physical_result_qualified=false` and `real_world_validation_status=UNRESOLVED`.

Schemas: `calculix-rr-normalization-manifest-v0.1.schema.json`, `physical-support-node-catalog-v0.1.schema.json`, and `calculix-rr-normalization-report-v0.1.schema.json`.

Neither function currently creates a solver receipt or authenticates execution. Before use in the equilibrium auditor, the caller must persist the raw deck, raw `.dat`, support-node catalogue and normalized output under an artifact root and declare their exact digests as receipt input/output artifacts. The existing byte verifier must check those artifacts before numerical scoring.

The parser intentionally does not call the values newtons. CalculiX input files have no intrinsic unit system; downstream normalization assigns SI units only because the exact request and manifest state that convention and the request is bound by digest—not because the RR output itself carries units.

## Adversarial controls

The Rust tests include:
1. A synthetic output with both ordinary `forces (fx,fy,fz)` and reaction-only `reaction forces (rfx,rfy,rfz)`; assert that the parser returns the latter values.
2. RF-only output with no RR section -> rejected.
3. RR output without an explicit RR request in the deck -> rejected.
4. Wrong set name, missing/extra/duplicate node IDs -> rejected.
5. Non-finite numbers, malformed exponent syntax, invalid UTF-8 -> rejected.
6. Fortran `D` exponent syntax -> parsed with finite values.
7. Duplicate RR sections -> rejected.
8. Exact input/output digests carried into deterministic report metadata.

These are synthetic parser tests, not evidence that any external CCX binary was executed.

## Required follow-on before a backend can qualify

1. Pin a candidate CCX source/build, executable SHA-256, platform and environment; prove the candidate supports RR with a controlled real run.
2. Run a rights-cleared, independently derived bar/cantilever case and preserve the original input deck, stdout/stderr, exit code, `.dat`, `.frd`, command line and executable/build provenance.
3. Independently map support node IDs to coordinates from a verified node catalogue derived from the exact deck/mesh.
4. Aggregate support force and moment about a declared origin without double-counting individual `r × F` terms and an already aggregated support moment.
5. Bind normalized outputs to the request, scene, frame, units and raw artifact bytes before they enter the equilibrium/reference/mesh gates.
6. Add release-specific real-output fixtures from the pinned candidate. Synthetic strings cannot establish parser compatibility with every CCX version.

## Claim ceiling

The module parses a narrowly specified RR result section. It does not run CCX, prove that the solver generated the file, normalize units/coordinates, establish equilibrium by itself, qualify a solver, validate a physical model, or authorize engineering use.