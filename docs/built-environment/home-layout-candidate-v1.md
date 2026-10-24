# Home Layout Candidate V1

- **Status:** bounded candidate geometry contract, not an architectural or engineering approval.
- **Version:** 0.1.0
- **Input:** [Home Design Brief V1](home-design-brief-v1.md).
- **Schema:** [home-layout-candidate.schema.json](home-layout-candidate.schema.json).
- **Fixture:** [example-home-layout-candidate.json](fixtures/example-home-layout-candidate.json).

## Purpose

Make the output of a future layout generator explicit and testable. Each candidate is linked to the exact resident-brief file bytes, accounts for every requested space via a zone or explicit omission, and represents every zone as a bounded rectangle on a named floor.

This is a deliberately narrow first geometry contract, not a full BIM model.

## Validate the candidate

From the Symtropy repository root:

```sh
cargo run -p symtropy-housing-evidence -- \
  --validate-layout docs/built-environment/fixtures/example-home-layout-candidate.json \
  --brief docs/built-environment/fixtures/example-home-design-brief.json
```

The report separately identifies schema validity, brief validity, exact brief linkage, space-accounting consistency, and basic geometry consistency. It also reports per-zone width, depth, rectangular area, optional resident target area, and area delta. These metrics are informational: rectangles do not encode wall thickness or usable floor area, and target areas are resident exploration values rather than code minima. Adjacency diagnostics are similarly informational and do not prove openings, circulation or acoustic/visual properties. Editing even the whitespace of the referenced brief changes its digest, so the candidate must be rebound and checked against that exact file revision.

## Space accounting and conflicts

Each zone references a requested `space_id` and a 1-based `instance_index`. The included instances must match the requested quantity. V1 does not support partial quantity: include the requested quantity or explicitly omit a non-must-have space with a reason.

A `must_have` space cannot be omitted or silently demoted. If the candidate cannot include one, validation fails and the system must preserve the resident brief and report infeasibility. Preferred and optional spaces may be omitted only with an explicit reason. A space cannot be both represented and omitted.

## Geometry rules

- Coordinates are local-plan Cartesian metres, X right and Y up on the plan view; they are not georeferenced.
- Every floor declares a planning rectangle. Every zone must have positive width and depth and lie inside that rectangle.
- V1 supports only `axis_aligned_rectangle_v1`; it does not encode walls, thickness, doors/windows, openings or circulation widths.
- Same-floor zone intersections are rejected when both intersection width and depth exceed 1e-9 m. This is a floating-point comparison tolerance only, not a construction tolerance or permission to overlap. A sliver no wider or deeper than that numeric tolerance may be treated as touching boundaries.
- Floor IDs and level indices must be unique, and zone-to-floor and zone-to-space references must resolve.

## Adjacency preference diagnostics

The report includes one diagnostic for each resident brief adjacency preference. For a connected preference, the validator may report a positive-length shared rectangle boundary as SHARED_BOUNDARY_PROXY_DETECTED, or report that no such proxy was found. A shared boundary is not evidence of a doorway, opening, accessible route, or circulation clearance. If either space is absent, the diagnostic reports SPACE_INSTANCE_MISSING.

Preferences such as near, separate, quiet_from, visual_connection_to, and outdoor_access_from are explicitly NOT_ASSESSED in this version. A minimum same-floor rectangle gap is emitted as an informational distance metric where matching zones exist, but no universal near threshold is assumed. These diagnostics do not change candidate validity.

## What a valid result does not mean

`LAYOUT_CANDIDATE_CONTRACT_VALID` means only that the candidate's schema, brief linkage, space accounting and simple rectangle checks passed. It does **not** prove room adjacency, circulation, accessibility, line of sight, acoustics, daylight, ventilation, thermal performance, structure, foundations, fire egress, building-code compliance, permits, cost, or buildability.

Those require separate versioned checks with declared methods and evidence. Symtropy's rigid-body engine must not be presented as a qualified structural or whole-building energy solver. Sol-Atlas context remains source- and resolution-qualified; Mycelix may coordinate consented records, but signatures alone do not establish physical truth.

## Test corpus

Rust unit tests include a synthetic valid rectangle plan and negative controls for brief digest mismatch, overlapping rectangles (including a small-area corner intersection), zones outside floor bounds, removal of a must-have space, and unrequested space IDs. The fixture also asserts numeric dimension/area metrics and confirms that a difference from the resident's target area is informational, not an automatic failure. This example is synthetic and makes no construction or compliance assertion.

## Provenance-bearing dimension constraints

Each optional `spaces[].dimension_constraints[]` entry specifies `constraint_id`, `axis` (`x_extent` or `y_extent`), `bound` (`minimum` or `maximum`), `value_m`, `origin`, and `enforcement`. The origin states where the requirement is said to come from; the enforcement value independently states whether this project contract treats it as advisory or must-satisfy. Only explicit `must_satisfy` constraints affect semantic validity. Diagnostics identify each room instance and the declared source metadata.

Professional or authority references must provide `source_reference`, `source_edition`, and `source_location`; this prevents an unsupported regulatory value from masquerading as a sourced input. It does not authenticate a link, verify the edition or clause, decide whether a rule applies, or establish legal compliance. The synthetic fixture's resident targets are examples only.

A reported extent is derived from the candidate's axis-aligned rectangle bounds. It is not a wall-face measurement, net usable room size, accessible clear width, nor an assessment of local building regulation. Keep the constraint unresolved or advisory until the underlying reference and applicability have been checked by a qualified reviewer.
