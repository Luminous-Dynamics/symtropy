---
title: Wood Products Lifecycle Reference Benchmark Suite
version: 0.1
status: proposed-reference-campaign
date: 2026-10-16
scope: synthetic, deterministic lumber yield, quality, provenance and lifecycle contracts
evidence_class: synthetic-fixtures-until-qualified-data-is-added
---

# Wood Products Lifecycle Reference Benchmark Suite v0.1

## Purpose

This suite turns improvements to woodworking and industrial lumber into falsifiable engineering work. It defines a small synthetic corpus and an independent Rust verifier for material accounting, quality gates, harvest constraints, custody integrity and lifecycle trade-offs.

The corpus is a **contract test**, not a lumber optimizer, forest inventory, sawmill digital twin, sustainability certification or estimate of commercial savings. All cost, energy and impact values are synthetic integer units. No species-specific process profile or live forest data is bundled.

## Paradigm shift

**Optimize qualified, durable service and recovered material value across the lifecycle — not board count, nominal yield or purchase price in isolation.**

A log is not simply a volume to maximize. It is an uncertain biological material with geometry, defects, species and moisture properties. A board is not a qualified product simply because a machine can cut it. Drying consumes energy and time; grading and inspection determine allowed uses; offcuts and sawdust have alternative paths; and product claims require custody and evidence.

This leads to six engineering shifts:

1. **Volume yield → qualified yield.** Measure recovered material by output stream, intended use, grade/inspection state, defects and downstream demand.
2. **Single best cut plan → constrained Pareto frontier.** Enforce mass, equipment, moisture, safety and provenance gates before comparing cost, cycle time, energy, grade yield or recovery.
3. **Static factory → uncertainty-aware production.** Species, geometry, moisture, defect and equipment inputs need sources and measurement states. Unknown data remains unknown rather than being replaced with an optimistic average.
4. **Waste as a disposal cost → residuals as traceable feedstocks.** Compare viable routes for chips, bark, offcuts and sawdust while maintaining material ledgers and preventing the same kilogram from being allocated twice.
5. **Origin label → event-linked custody.** Connect source, processing, drying, inspection, grading, sale, reuse and correction records without pretending that a signed claim proves the physical world.
6. **Map layer → decision context with limitations.** Spatial features carry source, capture time, coverage, resolution, coordinate reference, uncertainty and usage rights. Unknown coverage never means harvestable land.

## Project responsibilities

- **Sol-Atlas:** spatial and temporal context for forests or plantations, management/exclusion zones, transport links, mills, drying facilities and recovery sites. Expose provenance, coverage, resolution and freshness.
- **Symtropy:** deterministic material-flow accounting, sawing and drying simulations, mill capacity/energy constraints, lifecycle evaluation and Pareto alternatives. The first benchmark verifier is intentionally independent of the production simulation.
- **Symthaea:** bounded candidate-plan generation, hypothesis comparison and explanation. It cannot invent missing inputs, waive a hard constraint, certify grade or control physical machinery.
- **Mycelix:** signed provenance/custody claims, replay-safe event identity, selective disclosure when useful, and append-only correction/retraction links. A valid signature proves who asserted a claim, not that the claim is physically true.
- **Independent verifier:** checks exact corpus bytes, schema, units, arithmetic, feasibility gates, expected option sets, negative controls and the exact source revision.

## v0.1 scenario corpus

The frozen corpus contains six synthetic scenarios:

| ID | Required behaviour | Primary negative control |
|---|---|---|
| SAWING_DRY_MASS_CLOSURE | The sum of lumber, chips, sawdust, bark and declared loss equals input dry mass. | Reject mass creation and silent deletion of residuals. |
| STRUCTURAL_GRADE_GATE | Only options with qualified grade evidence and a passed inspection can serve a structural application; choose the best synthetic margin among eligible options. | A high-margin unverified option must not win. |
| DRYING_QUALITY_FLOOR | Minimize energy only among schedules with measured moisture inside the declared window and no declared defect. | Unknown moisture, excessive moisture and an over-drying defect are not feasible. |
| HARVEST_REGENERATION_GUARD | Require the synthetic harvest cap, zero overlap with excluded areas, qualified regeneration evidence and a passing ecological gate. | Cheap or high-yield harvest cannot override an exclusion or missing evidence. |
| CUSTODY_REPLAY_AND_CORRECTION | Simulate a lot's owner transitions, identical replay and linked correction; preserve one valid owner. | Two transfers from a stale owner cannot both commit; corrections do not erase history. |
| LIFECYCLE_RECOVERY_PARETO | Apply provenance, grade and qualified-output floors, then compute the Pareto frontier over synthetic cost, energy, impact and output. | Ineligible reclaimed material cannot win; a dominated feasible option is not on the frontier. |

All quantities are integers with explicitly named units. Moisture values use basis points (100 basis points = 1 percentage point). No floating-point tolerances are required by the v0.1 corpus. Real measurements may require an explicit conversion profile, instrument uncertainty and domain-specific tolerance; those are outside the synthetic validator's authority.

## Required validation order

1. Parse UTF-8 JSON and reject unknown or missing keys in every validated object.
2. Validate suite identity, version, units, evidence class and the exact unique scenario set.
3. Recompute material accounting and feasibility from raw fields instead of trusting expected result labels.
4. Apply quality, provenance and ecological gates before cost or margin comparison.
5. Recompute the selected eligible option and Pareto frontier with deterministic tie handling.
6. Emit a receipt that binds the exact corpus-byte SHA-256 and a caller-supplied 40-character source commit.
7. Fail closed if any check is indeterminate, malformed, overflowed or contradictory.

The verifier validates reference fixtures, not arbitrary optimization candidates. A future candidate runner must submit its own normalized outputs to a distinct checker and must not share its selection logic as the verifier's only oracle.

## Benchmarks and acceptance targets

### v0.1 contract targets

- 100% of mass-imbalanced fixtures are rejected.
- 100% of structural plans missing qualified grade evidence or a passing inspection are ineligible.
- 100% of drying schedules with unknown/out-of-range moisture or a declared defect are ineligible.
- 100% of options exceeding the synthetic harvest cap, intersecting an exclusion, lacking qualified regeneration evidence, or failing the ecological gate are ineligible.
- Replaying an identical event ID and payload is a no-op; replaying an event ID with a different payload is rejected.
- At most one transfer from the current owner is accepted; a stale-owner transfer is rejected.
- Corrections append a linked record and never silently rewrite custody history.
- Every ineligible lifecycle option is excluded before Pareto analysis; no strictly dominated feasible option remains on the frontier.
- The exact input bytes and source revision are bound into a deterministic receipt.
- Malformed schema, unknown units, duplicate scenario IDs, unknown object keys and arithmetic overflow fail closed.

### Future measured-engine targets

Do **not** claim mill efficiency gains until candidate planners have been benchmarked against representative, legally shareable and properly attributed data. A future report should include:

- recovered dry mass by product/grade/co-product stream;
- qualified output per incoming dry mass and per log;
- board-foot or cubic-metre recovery, with the unit and measurement method specified;
- drying energy per qualified unit, elapsed time, moisture distribution and defect/rework rate;
- sawline throughput, bottleneck utilization, downtime sensitivity and p50/p95 runtime;
- yield, grade mix and cost under species/log geometry/defect variance;
- recovered-residual fraction, landfill/disposal mass and double-allocation checks;
- provenance completeness, duplicate/replay detection and unresolved evidence rate;
- lifecycle cost and environmental indicators only where boundaries, sources, allocations and impact factors are documented.

Baselines must use the same input corpus, constraints and reporting units. Show raw metric vectors and Pareto alternatives rather than hiding a trade-off in one weighted score.

## Evidence and domain guardrails

The USDA Forest Products Laboratory's Wood Handbook is a starting technical reference for wood properties, moisture relations, lumber grades, drying, fastening, durability and fire performance: https://research.fs.usda.gov/fpl/wood-handbook. It is not a substitute for applicable grading rules, current codes, qualified inspectors or species-specific measurements.

The FAO Sustainable Wood for a Sustainable World initiative identifies sustainable wood value chains, value addition and circularity as relevant goals: https://www.fao.org/forestry/our-focus/forest-products/sustainable-wood-for-a-sustainable-world/en. A software benchmark does not establish that a specific harvest is sustainable, legal or beneficial to biodiversity.

Synthetic fixtures must not be described as real prices, real carbon factors, qualified structural lumber, certified sustainable sourcing or a real sawmill result. Any live-data adapter needs attributable sources, capture dates, measurement methods, units, uncertainty, data-licence review and jurisdiction-specific applicability. Human forestry, mill-safety and grading expertise remains necessary.

No benchmark output may actuate a saw, kiln, harvester, vehicle or industrial controller. This is research and decision support only.
