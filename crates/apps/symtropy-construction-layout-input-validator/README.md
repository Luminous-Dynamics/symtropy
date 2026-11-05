# Construction layout input-integrity validator

Pure-Rust, no-I/O pre-print gate for a CAD/BIM source/export manifest, coordinate-frame/transform binding, feature inventory and independent survey-checkpoint evidence.

## Run the synthetic reference example

```sh
cargo run -p symtropy-construction-layout-input-validator -- \
  --manifest research/construction-layout-input-v0-1/fixtures/valid/matched-input.json
```

The CLI embeds a synthetic demonstration policy. It is for deterministic fixture replay only. Production callers must use the library with a policy supplied by a trusted project/site authority and protected by that integration's authorization and provenance system.

## Decision model

- `admissible_for_named_layout_input`: all software-evaluable requirements of the named policy are met.
- `inadmissible`: explicit source/revision/frame/transform/inventory/checkpoint mismatch, or a measured residual plus U95 exceeds the declared decision margin.
- `indeterminate`: missing/insufficient survey evidence, unknown independence, weak checkpoint geometry or uncertainty too wide to resolve the margin.

The input manifest cannot submit its own decision or reason codes. Strict Serde parsing rejects unknown fields. The separate closed Draft 2020-12 manifest and trusted-policy schemas are in `research/construction-layout-input-v0-1/schema/`.

## Checked bindings

- Source model ID, revision, digest and approval reference.
- Export lineage to the exact source revision, exporter ID/version/configuration and export digest.
- Project/site/work-package/task/work-zone identity, site epoch digest and approved change sequence.
- Horizontal/vertical reference, coordinate epoch, source/target frames, units, axis order, handedness and exact transform bundle/resource digests.
- Exact multi-discipline revision set and critical feature inventory.
- Upstream survey-result and adjustment reference/digests; fit-point and independent-check-point identities remain disjoint.
- Independent checkpoint target-frame binding, reference-independence assessment, residuals, U95 and spatial spread.
- No manifest-authored feature exclusions in v0.1. The project cannot hide a missing critical entity by declaring it excluded.

## Boundaries

The evaluator does not fetch or hash source model bytes, authenticate signatures/issuers, validate surveyor competence, run a CRS transform, adjust a survey network, detect BIM clashes, prove design correctness, issue robot commands, mutate a worksite, or authorize printing. Referenced evidence is not independently authenticated by this crate. The upstream owners remain authoritative.

No new CRS engine, survey system, BIM clash engine, site ledger or safety controller is introduced. The implementation composes #1607, #3524, #3522, #3531, #3542 and #3558.

Fixtures and all values are invented. A passing synthetic software test does not establish physical survey accuracy, field safety, legal/professional approval, or standards conformity.
