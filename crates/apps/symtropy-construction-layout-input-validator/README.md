# Construction layout input-integrity validator

A pure-Rust, no-I/O pre-print gate for an exact CAD/BIM source/export manifest, frame/transform binding, feature inventory, and independent control checkpoints.

## Run the synthetic reference example

```sh
cargo run -p symtropy-construction-layout-input-validator -- \
  --manifest research/construction-layout-input-v0-1/fixtures/valid/matched-input.json
```

The CLI embeds one **synthetic demonstration policy**. It is only for fixture replay. Production callers must provide the library with a trusted, policy-controlled project/site/work-package profile from the owning authority. This crate does not authenticate that authority.

## Contract

The evaluator checks:
- exact source model revision and digest against the trusted project policy;
- exact export-to-source revision lineage, exporter identity/configuration and digest shape;
- current site, task, work-package, zone and worksite epoch binding;
- horizontal/vertical reference, coordinate epoch when required, units, axis order, handedness, source/target frame IDs and exact transform bundle/resource digests;
- exact discipline revision set and critical feature inventory across approved model and export;
- separation of fit-point IDs from independent check-point IDs;
- checkpoint count, uniqueness, spatial spread, claim-relative independence references, residual plus U95 decision margin and uncertainty ratio.

The evaluator never performs a coordinate transform, adjusts a survey network, authenticates signatures, fetches source bytes, verifies surveyor identity/competence, approves a design, detects BIM clashes, commands a robot, changes a worksite epoch, or certifies as-built work. It only evaluates the supplied manifest against the trusted policy object.

## Output semantics

- `admissible_for_named_layout_input`: the manifest meets this named policy's software-evaluable conditions.
- `inadmissible`: explicit source/revision/frame/transform/inventory/checkpoint conflicts or a measured residual beyond the declared margin.
- `indeterminate`: missing, insufficient, unverified or too-uncertain evidence prevents admission.

No universal PASS, composite score, global accuracy claim, or operational authorization exists.

## Ownership

Consumes, does not replace:
- #1607 surveying/frame/transform/calibration/uncertainty and independent checkpoints;
- #3523/#3524 layout performance/common-referee benchmark;
- #3522 physical trial receipt;
- #3542 worksite epoch invalidation/requalification;
- #3558 observation coverage;
- #1834 capability-to-work-package matching.

Fixtures and digests are invented. A synthetic software pass proves only the validator contract; it is not physical or legal survey evidence.
