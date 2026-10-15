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

The report separately identifies schema validity, brief validity, exact brief linkage, space-accounting consistency, and basic geometry consistency. Editing even the whitespace of the referenced brief changes its digest, so the candidate must be rebound and checked against that exact file revision.

## Space accounting and conflicts

Each zone references a requested `space_id` and a 1-based `instance_index`. The included instances must match the requested quantity. V1 does not support partial quantity: include the requested quantity or explicitly omit a non-must-have space with a reason.

A `must_have` space cannot be omitted or silently demoted. If the candidate cannot include one, validation fails and the system must preserve the resident brief and report infeasibility. Preferred and optional spaces may be omitted only with an explicit reason. A space cannot be both represented and omitted.

## Geometry rules

- Coordinates are local-plan Cartesian metres, X right and Y up on the plan view; they are not georeferenced.
- Every floor declares a planning rectangle. Every zone must have positive width and depth and lie inside that rectangle.
- V1 supports only `axis_aligned_rectangle_v1`; it does not encode walls, thickness, doors/windows, openings or circulation widths.
- Same-floor zones may not overlap by more than (10^{-6}\,m^2). The tolerance is numeric comparison only, not a construction tolerance.
- Floor IDs and level indices must be unique, and zone-to-floor and zone-to-space references must resolve.

## What a valid result does not mean

`LAYOUT_CANDIDATE_CONTRACT_VALID` means only that the candidate's schema, brief linkage, space accounting and simple rectangle checks passed. It does **not** prove room adjacency, circulation, accessibility, line of sight, acoustics, daylight, ventilation, thermal performance, structure, foundations, fire egress, building-code compliance, permits, cost, or buildability.

Those require separate versioned checks with declared methods and evidence. Symtropy's rigid-body engine must not be presented as a qualified structural or whole-building energy solver. Sol-Atlas context remains source- and resolution-qualified; Mycelix may coordinate consented records, but signatures alone do not establish physical truth.

## Test corpus

Rust unit tests include a synthetic valid rectangle plan and negative controls for brief digest mismatch, overlapping rectangles, zones outside floor bounds, removal of a must-have space, and unrequested space IDs. This example is synthetic and makes no construction or compliance assertion.
