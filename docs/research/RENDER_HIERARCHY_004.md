# RENDER-HIERARCHY-004 — Deterministic hierarchy selection for solar-system scale

**Status:** Draft policy kernel. Thirteen Rust unit tests are authored; exact-head tests have not yet qualified this module.

## The missing bridge between large-world coordinates and a renderer

Large-world coordinates prevent precision from collapsing as the camera moves away from a planet. Apparent-size bands decide which type of presentation is suitable at a given screen size. Neither alone chooses a complete hierarchy of body proxies, surface regions, local engineering detail, and astronomical catalog representations.

This tranche adds a deterministic hierarchical representation selector. The host supplies camera-derived pixel error and content-residency status. The selector chooses stable representation nodes while preserving canonical semantic entity identity.

## Contract

Each node declares:
- stable representation-node ID and canonical semantic-entity ID;
- parent and child IDs;
- source content state (ready, loading, or unavailable);
- replacement or additive refinement;
- a statement that the immediate children completely cover the parent, required for replacement;
- screen-space error supplied by the host;
- optional forced refinement / detail protection for content that cannot be coarsened when refinement is required.

The selector validates the tree as a forest: no duplicate node or child IDs, all parent/child links must match, no orphaned nodes, and no cycles. Output is sorted by stable ID to make results independent of input/ECS ordering.

### Replacement refinement

If the parent's screen-space error exceeds the policy ceiling, all immediate children must be resident and certified to cover the parent's represented content before the parent can be replaced. If any child is loading but a ready parent exists, the parent is held and the fallback is recorded. If the parent is also unavailable, the selector fails rather than drawing a hole and pretending the region is empty.

### Additive refinement

When additive refinement is requested and all children are resident, the selector keeps the ready parent and renders the children. If child content is still loading, a ready parent remains visible and the incomplete refinement is recorded.

### Critical/detail-pinned requirements

A host may detail-pin a node when a task genuinely requires finer information. If a requested refinement cannot be supplied, the selector returns an error instead of silently relaxing the requirement. This should not be applied to every selected astronomical body: a far-away selected body can remain a point/disc/proxy while preserving its semantic ID and selection mapping. Pin detailed geometry only when the task requires it.

## Relationship to established approaches

OGC 3D Tiles defines geometric error for the difference between a tile and its source representation, and screen-space error in pixels; it uses hierarchical levels of detail, bounding volumes and replacement/additive refinement. Symtropy should reuse that mature hierarchy vocabulary rather than claim invention of HLOD. The extension here is the combination with the preceding semantic-protection policy and explicit fail-closed content availability.

- OGC 3D Tiles 1.1: https://docs.ogc.org/cs/22-025r4/22-025r4.html
- OGC 3D Tiles 1.0 geometric-error/coverage definitions: https://docs.ogc.org/cs/18-053r2/18-053r2.html

## Frozen solar-scale traversal fixture

The companion manifest `benchmarks/render/SOLAR-SCALE-01.json` defines seven scale stages and seven proposed camera waypoints, from a local feature at 5 m to the inner system at 2 AU and an outer-system overview at 40 AU, then back to the original feature. It also specifies eight hierarchy-selection cases and reference body/distance values.

These waypoints are explicitly marked illustrative geometry, not a solar-system ephemeris. Their proposed near/far clipping values are workload inputs for a depth-strategy study, not recommendations to use one depth range for every stage. The full renderer must compare near/far partitions, reverse-Z and any logarithmic/scale-aware depth path with actual GPU captures and depth-precision/error receipts.

## Authored tests

Thirteen tests exercise:
- error below budget retains the parent;
- error above budget selects children and deterministic order;
- loading replacement children hold a ready parent;
- detail-pinned content fails when required child detail is unavailable;
- uncertified replacement coverage is rejected when forced;
- additive refinement keeps parent plus child;
- forced refinement overrides an otherwise small screen error;
- an unavailable parent can be bypassed only by ready children with full-coverage declaration;
- invalid screen error and malformed parent-child links;
- cycles, duplicate child IDs and missing fallback content;
- results do not depend on input order;
- semantic identity remains stable across representation switches.

## Next steps

1. Pass formatter, focused render-bridge tests, strict Clippy, and workspace gates on the exact head.
2. Connect the selector to actual Bevy representation entities and source-content residency; avoid a second/forked renderer.
3. Bind body/region/tile bounds and screen-space errors from qualified camera/geometry providers.
4. Integrate temporal hysteresis and preserve selected-object identity across parent/child substitutions.
5. Run the frozen SOLAR-SCALE-01 traversal and receipts; report proxy/child counts, missing-content fallbacks, pops/chatter, GPU/CPU timing and physical-state invariance.

This is a policy kernel, not a complete hierarchical solar-system renderer. It does not yet implement surface tile generation, frustum culling, shader/asset streaming, authoritative ephemerides, GPU transforms or an end-to-end flight scene.
