# PCB design-skeleton invariant v0.1

**Status:** Rust implementation and synthetic tests; exact-head CI is still required. The parser is a conservative structural guard, not a full semantic-equivalence proof.

## Threat model

An apparent copper-routing improvement is invalid when it was obtained by changing the placed board, pad/net assignments, layer setup, board outline, design rules or zone constraints. Routing only changes copper connectivity; it must not silently modify the object being routed.

## Fingerprint construction

The evaluator parses the native KiCad board as an S-expression and hashes a deterministic serialization after excluding only explicitly recognized routing output:

- Root-level copper segment nodes, which must have start/end/width/layer/net attributes.
- Root-level via nodes, which must have at/size/drill/layers/net attributes.
- Root-level net-assigned routed arc nodes, which must have start/mid/end/width/layer/net attributes.
- Generated filled_polygon children within zone definitions, preserving the zone's net/layer/settings/polygon definition.

A recognized but incomplete routing primitive fails closed instead of being silently removed. Unknown objects are preserved. The receipt binds raw board SHA-256, computed design-skeleton SHA-256, excluded primitive counts and the exact evaluator Git HEAD.

## External-manifest and output gates

For external benchmark manifests (schema v0.2), every board must include a source-derived design_skeleton_sha256. The manifest builder computes it from the exact source-board bytes. verify_dataset_files recomputes and checks it. verify_run_design_skeletons then:
1. Verifies source/license/split bytes against the manifest.
2. Verifies every completed routed-output board and DRC file against the run report's actual byte digests.
3. Computes both skeletons and requires the routed output to match the pinned source skeleton.
4. Emits a receipt with both raw board hashes, both skeleton hashes and counts of stripped route primitives.

The compare-verified command includes these receipts for both baseline and treatment. A mismatch causes the verified comparison to stop; it must not be scored as a routing success. The standalone CLI form is:

    export PCB_ROUTE_BENCHMARK_GIT_HEAD="$(git rev-parse HEAD)"
    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- verify-run-skeletons REPORT.json MANIFEST.json DATASET_ROOT RUN_ROOT

The generic synthetic normalized-report comparator remains synthetic-only. Real external comparisons must use the verified path.

## Synthetic regression tests

The tests prove that (a) adding/removing track segments, vias and net-assigned route arcs, and changing generated zone-fill polygons leave the skeleton hash stable; (b) movement of a footprint, pad/net changes, net-class clearance changes and outline changes change the hash; (c) malformed route primitives, malformed expressions and non-KiCad roots are rejected; and (d) an end-to-end verified run accepts route-only edits but rejects a placement change even when the mutated output's SHA-256 is truthfully updated.

## Deliberate limits

This is not a complete KiCad semantic comparison or a legal statement of design intent. The current S-expression normalizer preserves all non-excluded nodes and list ordering; it may conservatively reject a semantically equivalent file if the serializer reorders nodes. Such a case needs a documented canonicalization extension and tests, not automatic acceptance.

A matching skeleton is not DRC compliance, electrical connectivity, controlled impedance, SI/PI, EMC, manufacturing quality or physical qualification. Raw DRC must still run against a pinned policy. Source-rule fidelity is separately tracked in PCB-DRC-FIDELITY-001 #2626.
