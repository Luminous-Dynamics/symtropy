# BIOPROC-CLOSURE-001 — Full-process engineered-protein closure benchmark

**Status:** synthetic-first benchmark contract; no measured biological, production, safety, economic, or lifecycle result is established.  
**Version:** 1.0.0  
**Created:** 2026-10-18  
**Related issue:** [Symtropy #2027](https://github.com/Luminous-Dynamics/symtropy/issues/2027)

## Research motivation

Two 2026 papers make a strong case that cultivated-protein analysis must go beyond cell-growth yield:

- A full-process model in *Frontiers in Nutrition* (29 June 2026) identifies cell expansion, media preparation/sterilization, cleaning-in-place, and cooling as important scale-up bottlenecks. It is a reference model with explicit assumptions, not an experimentally validated commercial plant design: https://doi.org/10.3389/fnut.2026.1844185
- A June 2026 *Bioresource Technology* review describes computational fluid dynamics as a rational tool for scale-up while emphasizing remaining gaps between modeled hydrodynamics, scale and biological validation: https://doi.org/10.1016/j.biortech.2026.134418

The paradigm shift is **from maximizing growth within the vessel to maximizing usable, specification-compliant output per complete process cycle**. The system boundary must include upstream preparation, culture/fermentation, support utilities, harvesting/recovery, rejection/losses, cleaning and turnaround.

This follows the evidence/claim separation defined by [Symthaea BIOPROTEIN program #8062](https://github.com/Luminous-Dynamics/symthaea/pull/8062). It composes with the existing [Symtropy exact resource/nutrient accounting PR #1637](https://github.com/Luminous-Dynamics/symtropy/pull/1637), and does not replace that accounting kernel.

## Scope

This corpus defines resource and evidence semantics for a declared process model. It is not a culture recipe, cell-growth model, bioreactor control system, safety determination, regulatory opinion, or industrial cost prediction.

The first target is product-agnostic accounting that can later be instantiated separately for microbial biomass, precision fermentation inputs, cultivated animal cells, and hybrid products. These production classes must not inherit assumptions from one another without justification.

## Declared process boundary

The standard process profile includes:

1. Feedstock/media preparation.
2. Sterilization or declared contamination-control process.
3. Cultivation or fermentation.
4. Utilities and thermal management.
5. Harvest and separation.
6. Downstream recovery and quality qualification.
7. Cleaning and turnaround.
8. Rejected-batch, off-spec, and process-loss allocation.

A stage can be marked not_applicable only when that exclusion is declared and justified for the exact process profile; it must not disappear from the denominator silently.

## Exact resource-accounting semantics

For each independently conserved resource stream in its declared base unit:

**inflow + additions − outflow − loss = observed_storage_delta**

The balance passes when the absolute integer difference is no greater than the declared integer tolerance. Each resource type fixes its base unit: grams (material), millilitres (water), watt-hours (energy), and seconds (time). Different resource kinds, units, and time bases cannot be added implicitly.

The first corpus encodes integer quantities as decimal strings to prevent accidental precision loss across JSON/JavaScript consumers. This is a representation contract, not proof that an observed flow is correct.

For the gross product mass split:

**gross_product = usable_product + rejected_product + downstream_recovery_loss**

A process that reports only gross biomass cannot claim usable-product yield. Contamination/rejected batches, off-spec output, cleaning/turnaround, and spent/wash streams stay visible if they affect the selected boundary.

## Disposition model

- eligible_for_declared_comparison: the synthetic fixture passes the narrow accounting and evidence gates for its declared claim.
- blocked: a hard contradiction or declared failed qualification prevents the requested claim.
- indeterminate: required evidence, boundary coverage, or scale-transfer qualification is missing or unresolved.
- not_comparable: units, boundaries, time bases, or scenario identities do not match.
- invalid_evidence: the fixture's source/evidence label contradicts its declared status, or a value violates the wire contract.

These dispositions apply only to the declared synthetic fixture. Even eligible_for_declared_comparison does not establish a real biological process, food safety, regulatory authorization, price competitiveness, or environmental superiority.

## First benchmark metrics

Report separately:
- number of required process stages covered versus unknown/incomplete;
- exact closure residual for every required resource;
- usable product per input and per complete cycle when all terms exist;
- recovery loss, rejected product, turnaround duration, and facility utilization where represented;
- utilities and cost per usable product under stated scenario assumptions;
- explicit unknown/estimated/synthetic shares, plus scenario-level rank changes.

Do not hide an unclosed mass/resource account or a missing safety/quality gate behind an aggregate process score.

## Frozen synthetic corpus

The companion corpus contains 12 cases, including:
- exact-closure positive control;
- required CIP/turnaround stage omitted;
- gross mass reported without usable-product breakdown;
- rejected-batch burden omitted;
- incompatible units;
- required energy balance missing;
- unknown residual stream;
- unqualified scale extrapolation;
- CFD mislabeled as biological validation;
- scenario digest mismatch;
- incorrect resource closure;
- measured-vs-synthetic evidence-kind mismatch.

Fixtures are entirely synthetic. Values are small hand-checkable integers, not suggested culture conditions, real yields, current prices, or experimental observations.

## Acceptance gates

1. Parse both JSON schemas and the corpus; reject unknown fields, duplicate IDs, unsupported schema versions, count mismatches, absent units, empty required provenance, and non-integer wire quantities.
2. Achieve 100% exact agreement with independent hand-computed balance and disposition fixtures.
3. Zero silent imputation of missing balance streams, stage coverage, usable product, rejected batches, or process losses.
4. Input/stage ordering must not change decisions or canonically ordered receipts.
5. Bind each receipt to exact corpus bytes (SHA-256), schema version, scenario and source-assumption digests, repository HEAD, exact attempted/passed/failed/skipped counts, and per-fixture expected/actual result.
6. Missing fixtures, wrong head/schema, duplicate stream identifiers, or a failed negative control must produce incomplete or fail, never pass.
7. Model outputs must not be represented as measured biological productivity or validated scale-up unless the corresponding independent biological evidence exists.

## Next implementation

- Compose with existing Symtropy resource accounting, using a separate oracle rather than the calculator's own decision function.
- Add Rust types/validation and a receipt runner only after the corpus contract is reviewed.
- Use synthetic fixtures for mechanics. Do not include proprietary feed media composition, unlicensed datasets, organism-engineering details, or physical control setpoints.
- Add real process data only with permission, measurement units, method, scale, batch identity, uncertainty, and reviewable source provenance.
