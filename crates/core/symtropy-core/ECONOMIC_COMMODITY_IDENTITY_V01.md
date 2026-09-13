# ECON-04 — Economic Commodity Identity v0.1

Status: frozen source contract for qualification

## Purpose

ECON-04 gives the ECON-00/01 `CommoditySpecId` an explicit, deterministic economic meaning without turning the economic kernel into a materials-science database, process-recipe authority, or manufacturing controller.

The layer binds live stock to exact quantity-unit semantics, commodity specification identity, optional grade/quality identity, optional opaque physical-material evidence references, and an explicit tracking mode for fungible, batch-tracked, and serialized stock.

## Governing theorem

For a validated `StockLedger` `S` and validated `CommodityIdentityRegistry` `R`:

```text
live_lot_ids(S) == metadata_lot_ids(R)

for every live lot L in S:
    L.commodity_spec_id == metadata(L).commodity_spec_id
    metadata(L).commodity_spec_id exists in R.specs
    R.specs[L.spec].quantity_unit_id exists in R.units
    metadata(L).tracking_identity matches R.specs[L.spec].tracking_mode
```

No live lot may lack semantic metadata and no metadata record may claim a lot that is not live.

## Quantity semantics

Every registered `QuantityUnitDefinition` binds:

- one `QuantityUnitId`;
- one `QuantityDimensionId`;
- an exact positive reduced rational scale to the canonical unit of that dimension.

ECON-04 uses integer/rational arithmetic only for unit conversion. Floating-point conversion is not an authority path.

For compatible dimensions:

```text
amount_to = amount_from
          * from.canonical_numerator
          * to.canonical_denominator
          / from.canonical_denominator
          / to.canonical_numerator
```

The conversion is admitted only when the exact result is integral and representable as `u64`. Fractional results and arithmetic overflow fail closed. Cross-dimension conversion fails closed.

Unit conversion is calculation only. It has no authority to mutate a stock lot, change a `CommoditySpecId`, create/deplete stock, or reclassify material.

## Commodity specification identity

A `CommoditySpecDefinition` binds one `CommoditySpecId` to:

- exact quantity unit;
- tracking mode;
- optional `CommodityGradeId`;
- optional `QualitySpecificationId`;
- optional opaque `PhysicalMaterialReference`.

Grade and quality are identities, not free-form mutable annotations. Changing the economically authoritative commodity specification requires an explicit stock transformation under the appropriate upstream authority; ECON-04 provides no generic `set_grade`, `relabel`, `reclassify`, or specification-mutation operation.

## Tracking modes

### Fungible

Lot identity is operational. No persistent batch or serialized-asset identity is claimed by ECON-04 for that lot.

### BatchTracked

Every live lot must carry a `BatchId`.

A physical/economic lot split may retain the same batch identity on both resulting lots. Splitting warehouse inventory therefore does not fabricate a new manufacturing batch.

ECON-04 does not itself establish what physical process created a batch.

### Serialized

Every live lot must carry an `AssetId` and have economic quantity exactly `1` in its registered commodity unit.

Within one validated live registry, a serialized `AssetId` may appear at most once. This prevents two simultaneously live economic lots from claiming authority over the same serialized asset.

ECON-04 does not establish the physical existence, geometry, condition, telemetry, or engineering properties of that asset.

## Quality evidence

If a commodity specification declares a `QualitySpecificationId`, each live lot under that specification must carry a `quality_evidence_id`.

If a commodity specification does not declare a quality specification, attaching `quality_evidence_id` is rejected. Evidence cannot become authority merely by being present.

Every live lot also carries a provenance evidence reference explaining why it is entitled to the declared commodity specification. ECON-04 validates the identity binding, not the truth of the upstream evidence.

## Physical-material bridge

`PhysicalMaterialReference` is deliberately opaque:

```text
MaterialAuthorityId
PhysicalMaterialId
CausalId evidence
```

Its presence does not assert density, composition, phase, strength, chemistry, purity, thermal behavior, recipe validity, or any other physical/material property. Those claims remain owned by a future or external physical-material authority.

## Industrial-ecology boundary

Existing `IndustrialEcology` inventory is expressed in generic qualified `u64` units. ECON-04 MUST NOT silently reinterpret those units as kilograms, litres, items, energy, or another quantity dimension.

Any future IndustrialEcology ↔ ECON-04 bridge must provide an explicit evidence-bound unit/specification mapping. String equality between an industrial dependency ID and a commodity/unit ID is insufficient authority.

## Immutability boundary

`CommodityIdentityRegistry` v0.1 exposes no public semantic mutator. A changed stock state, commodity catalog, quality binding, or lot metadata set must be validated into a new registry snapshot.

This deliberately prevents in-place semantic drift from bypassing the stock ledger's append-only causal history.

## Determinism and bounds

Canonical maps use ordered keys. IDs are non-empty, bounded, and may not contain surrounding whitespace. Unit scales must be positive reduced fractions.

The registry applies explicit upper bounds to unit, specification, and live-lot metadata counts. Arithmetic conversion is checked and fail-closed.

## Required adversarial corpus

Qualification must include at least:

- complete fungible registry acceptance;
- missing/stale live-lot metadata rejection through exact coverage;
- unknown stock specification rejection;
- tracking-mode mismatch rejection;
- batch identity surviving a stock split;
- serialized quantity greater than one rejection;
- duplicate live serialized `AssetId` rejection;
- required quality evidence rejection when missing;
- unexpected quality evidence rejection when no quality specification exists;
- exact same-dimension rational conversion;
- fractional conversion rejection rather than rounding;
- cross-dimension conversion rejection;
- zero/non-reduced unit-scale rejection;
- arithmetic overflow rejection where applicable.

## Explicit non-claims

ECON-04 does NOT establish:

- physical mass/energy conservation between unlike commodity specifications;
- manufacturing recipes or process feasibility;
- mining, refining, chemistry, metallurgy, agriculture, or fabrication authority;
- material-property truth;
- automatic mapping from `IndustrialEcology` dependency units to economic units;
- packaging/container semantics;
- perishability, spoilage, decay, contamination, mixture, or blending rules;
- lot genealogy across arbitrary transformations;
- warehouse reservations, escrow, sales, pricing, markets, or contracts;
- freight, routes, travel time, transport capacity, or delivery;
- physical serialized-asset state;
- legal title beyond the owner/custodian semantics already established by ECON-00/01;
- cross-partition commodity-catalog distribution or consensus.

Those require later typed authorities rather than being inferred here.

## Relationship to the ECON stack

```text
ECON-00/01 stock authority
        ↓
ECON-02 finance / settlement
        ↓
ECON-03 resolution + partition conservation
        ↓
ECON-04 commodity/unit/grade/batch/asset semantics
        ↓
future ECON-05 spatial logistics
```

ECON-04 enriches the meaning of already-authoritative live stock. It does not bypass or replace the stock ledger.

## Qualification rule

Source review, static audit, or PR mergeability alone are not executable qualification.

A qualifying lane must be frozen to the exact ECON-04 product head and exact ECON-03C parent, execute the ECON-04 corpus, regress all prerequisite ECON authority tests, run formatting/check/test/strict-Clippy gates required by the repository, and preserve machine-readable evidence sufficient to distinguish subject failure from qualifier/infrastructure failure.
