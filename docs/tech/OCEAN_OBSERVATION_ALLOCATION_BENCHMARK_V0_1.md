# OCEAN-OBSERVATION-ALLOCATION-001 — benchmark v0.1

**Status:** executable synthetic benchmark specification; no real ocean observations are acquired or assessed.

## Research motivation

Marine biodiversity evidence is not uniformly sampled across geography, depth, or habitat. A 2026 Nature Communications synthesis of roughly 48 million observations across 184,141 species explicitly accounted for sampling effort and spatial autocorrelation, finding substantial gaps and persistent spatial/depth biases. Those global findings motivate bias-aware survey design; they are not a CCZ-specific observation inventory. Source: [Saeedi et al., 2026](https://doi.org/10.1038/s41467-026-73613-z).

A 2025 paper prioritises collection to address geographic and benthic/pelagic marine data gaps: [Communications Earth & Environment](https://www.nature.com/articles/s43247-025-02442-7). Current deep-seabed regulatory standards and guidelines continue evolving, so any later operational constraints must be tied to exact dated instruments rather than assumed from an old draft: [Frontiers in Ocean Sustainability, 2026](https://doi.org/10.3389/focsu.2026.1838030).

GEBCO's global 15-arc-second grid is useful as broad spatial context, but its TID differentiates data-source classes and the output spacing must not be mistaken for direct measurement resolution. GEBCO says the grid must not be used for navigation or safety-at-sea purposes: [GEBCO_2026 Grid/TID documentation](https://www.gebco.net/data-products-gridded-bathymetry-data/gebco2026-grid), [TID definitions](https://www.gebco.net/gebco-tid-grid).

## Paradigm shift

The conventional question is: “Where do we already have many records, and what is the next easiest sample?” This benchmark instead asks: **Which permissible non-contact observation would most responsibly reduce declared evidence gaps without hiding sampling bias, detection limits, exposure, or rights uncertainty?**

Key separations:
- Historical record count is not abundance or ecological condition.
- No record is not evidence of absence.
- Grid spacing is not source measurement accuracy.
- More information gain does not override any individual exposure cap.
- An unresolved terms review is not treated as permission to acquire/redistribute data.
- A reproducible plan is not permission to actuate a vehicle, contact the seabed, collect specimens, or extract minerals.

## Input contract

The frozen corpus is strictly deserialized (deny_unknown_fields) and declares four synthetic scenarios. Each has:
- strata with declared priority, initial synthetic uncertainty, historical record count and historical sampling effort;
- candidate observation actions with mode, terms-review state, declared detection limits, exposure knowledge, effort, expected synthetic information gain and separate sound/light/sediment exposure;
- an effort budget and one independent cap for each exposure channel.

A candidate is eligible only if its mode is non-contact, its terms are reviewed, detection limits are declared, and exposure is known. Unknown exposure is excluded, not coerced to zero. Every selection step checks the total effort and all exposure channels independently.

## Comparison policies and objective

1. Fixed-seed deterministic pseudo-random selection.
2. Largest coverage gap.
3. Raw historical record count (deliberately weak anti-baseline).
4. Effort-aware stratification.
5. Greedy declared uncertainty reduction per effort.
6. Exact exhaustive subset oracle for each small scenario.

The synthetic **information score** is the sum, across selected observations, of marginal uncertainty reduction (capped by remaining stratum uncertainty) multiplied by declared stratum priority and divided by 1,000,000. Priority is a benchmark parameter, not an inferred ecological value. The **weighted coverage** metric is the sum of priorities for represented strata divided by total priority. Report these as separate metrics; neither is an environmental-safety score. The exact oracle maximizes the declared information-score objective, with weighted coverage and effort used only as explicit tie-breakers.

## Frozen acceptance gates

The tool's receipt checks:
- no selected contact action;
- no selected action with unresolved/restricted terms;
- no selected action with unknown detection limits or unknown exposure;
- each policy's effort and sound/light/sediment totals independently stay within limits;
- the exact oracle has zero regret and no other policy scores above it;
- effort-aware coverage exceeds raw-record-count coverage in the frozen access-biased scenario;
- the schema contains no species-presence/absence or ecological-safety conclusions.

The goal is 100% deterministic replay for the same exact corpus, tool/schema versions, seed and source commit. These are contract tests over synthetic worlds—not evidence that a specific sampling policy is ecologically optimal in the real ocean.

## Planned next stage

After this benchmark passes at an exact head, add explicit replicate/temporal observations, sensitivity sweeps over stratum priorities and measurement detection probability, and Pareto reports (coverage versus information score versus effort and each exposure channel). Only later should reviewed, appropriately licensed source metadata be used; a physical observation or environmental model must be independently qualified before it may affect operational decisions.

## Traceability and integration

This is a follow-on to [Symtropy CCZ observability #2011 / PR #2014](https://github.com/Luminous-Dynamics/symtropy/pull/2014), [Sol-Atlas CCZ Commons #165 / PR #166](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/166), and [Symthaea's marine experiment selection #7641](https://github.com/Luminous-Dynamics/symthaea/issues/7641). It uses synthetic input until those evidence and provenance layers expose a reviewed, source-pinned interface; it does not create a second provenance graph or canonical environmental-state owner.
