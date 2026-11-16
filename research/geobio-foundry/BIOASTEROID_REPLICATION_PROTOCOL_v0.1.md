# BioAsteroid Replication and Extension Protocol v0.1

**Date:** 2026-11-16  
**Status:** Research protocol; no experiment performed by Luminous Dynamics  
**Purpose:** establish a reproducible comparison of biological, abiotic and hybrid mineral-processing routes without overinterpreting a small orbital proof of principle.

## 1. Why replication comes first

The 2026 BioAsteroid paper reports ISS microgravity leaching of 44 elements from L-chondrite meteorite samples. Its headline result for *Penicillium simplicissimum* is a palladium leachate result around 5.5 times its abiotic control, but the result has substantial variability and limited replicate counts.

The published study used meteorite aliquots around 0.79 ± 0.14 g, fragments approximately 1–3.5 mm, and 5 mL of 50% R2A nutrient medium per sample. The medium contained organic nutrients and mineral salts. This is not a nutrient-independent asteroid biofactory, and leachate concentration is not the same thing as purified recovered metal.

This protocol therefore prioritizes **faithful reproduction, controls, and full process accounting** before organism optimization or extrapolation to asteroid-scale production.

Primary source: https://www.nature.com/articles/s41526-026-00567-3

## 2. Claims to test

Pre-register three separate questions:

1. **Reproduction:** under conditions matching the published protocol as closely as practicable, can the direction and uncertainty of the reported biological-versus-abiotic element-release effects be reproduced?
2. **Mechanism:** which differences are attributable to organism identity, viable biomass, culture medium, mineral batch heterogeneity, or environmental condition?
3. **Process utility:** after product separation and purification, does a biological or hybrid route improve useful recovered product per total feedstock, medium, water, energy, reactor time, and waste burden?

Do not conflate a statistically detectable leachate response with an industrially useful extraction process.

## 3. Study design

### Phase A — protocol reconstruction

Before a new experiment:
- Extract the full methods, supplementary material, sample IDs, ICP-MS/ICP-OES methods, blank corrections, detection limits, exclusions, and statistical analysis.
- Build a machine-readable table mapping every sample to organism, mineral batch, environment, medium, incubation duration, hardware unit, analytical run, and available controls.
- Reproduce published summary statistics from the reported data where available.
- Record all ambiguities as unresolved rather than filling gaps with assumptions.

### Phase B — matched controls

The exact control set must be finalized by a qualified microbiology/geochemistry laboratory after reviewing the original methods and biosafety requirements. The minimum analytical logic should distinguish:
- mineral + live organism + defined medium;
- same mineral + abiotic medium control;
- medium-only analytical blank;
- mineral-only control;
- organism/medium background control where the analytical method requires it;
- independent reference material and calibration standards for elemental analysis.

Additional killed-biomass or organism-specific controls may help distinguish passive sorption, cell-surface effects, and metabolism, but must be justified for the organism and analytical method. Use sterile/contained procedures, validated decontamination, and institutional biosafety review. This document is not a wet-lab protocol.

### Phase C — replication and statistical power

Do not copy the original small replicate count as if it were sufficient for industrial inference.

1. Use a pilot only to estimate batch variance, between-unit variance, analytical uncertainty, and plausible effect size.
2. Select confirmatory sample size through a preregistered power analysis for a small set of primary endpoints.
3. Treat mineral batch, experimental unit, and analytical batch as explicit sources of variation.
4. Randomize sample allocation and blind analytical labels where possible.
5. Define missing-data, contamination, failed-reactor, and below-detection-limit handling before unblinding.
6. Correct for multiple comparisons across the many elements. Do not select the most favorable element after seeing results and present it as the primary endpoint.

A mixed-effects model may be appropriate, with treatment and environment as fixed effects and mineral batch, hardware unit, and analytical batch as random effects where the design supports them. If sample size is too small for this model, report the limitation and use a simpler preregistered analysis.

## 4. Primary and secondary endpoints

### Primary scientific endpoints
- Element-specific released mass normalized to initial assayed element inventory (fraction of inventory released).
- Difference between biological and matched abiotic control for pre-registered target elements.
- Confidence intervals and between-batch variability.

### Primary process endpoints
- Purified recovered mass of on-spec product, not just concentration in leachate.
- Recovery fraction after solid/liquid separation and downstream purification.
- Total input burden per unit recovered product.

### Mandatory process accounting
Record separately:
- mineral feed mass, mineralogy, particle-size distribution, and assay;
- culture medium mass and volume, including all organic and inorganic constituents;
- water used, recovered, purged, or retained;
- gross electrical and thermal energy, plus recovered energy separately;
- reactor and consumable mass, sterilization burden, and operating time;
- separation and purification reagents, energy, yield and waste;
- final product identity, purity, residual contaminants, and disposition;
- batch failures, contamination events, downtime, and maintenance.

Use explicit functional units such as:
- mg recovered element per kg mineral feed;
- mg purified on-spec product per kg total dry input;
- mg purified product per kWh;
- mg purified product per litre process water;
- mg purified product per reactor-day;
- kg imported nutrient medium per kg purified product.

Do not report only mg/L, since concentration can increase through lower liquid volume without increasing total recovered mass.

## 5. Biological, abiotic and hybrid route comparison

For the same homogenized mineral batch, evaluate:
1. **Abiotic route:** a conventional matched chemical extraction selected by a qualified process chemist.
2. **Biological route:** the reproduced biological process.
3. **Hybrid route:** an explicitly defined sequence where biological conditioning or leaching is followed by an established separation step, or vice versa.

All three routes must use the same product specification and accounting boundary. Include pre-treatment, feedstock conditioning, medium/reagent preparation, heating/cooling, separation, purification, waste handling, and downstream product testing.

A biological route wins only if its recovered on-spec product per constrained resource improves beyond combined uncertainty, or if it delivers a separately quantified mission benefit that the abiotic route cannot provide. “Biological” is not itself an advantage.

## 6. Microgravity and destination transfer

The ISS experiment provides evidence only for the tested microgravity conditions, hardware, mineral samples, organism, medium and duration. It does not validate performance in lunar gravity, Martian gravity, asteroid surface conditions, or long-duration industrial reactors.

Future transfer studies should treat gravity, mixing, gas/liquid/solid transport, sedimentation, wetting, radiation, temperature, and reactor geometry as separate explanatory variables. Do not assume an Earth or ISS response transfers by a simple gravity scaling factor.

## 7. Statistical reporting

For each primary target element report:
- number of independent biological and abiotic experimental units;
- technical replicates separately from independent biological replicates;
- mean/median, dispersion, and confidence interval;
- effect size and uncertainty;
- raw concentration and calculated total released mass;
- initial feedstock inventory and material balance residual;
- analytical detection and quantification limits;
- correction for multiple comparisons;
- all exclusions and missingness.

Publish the raw and processed data, analysis code, sample map, instrument calibration, protocol deviations, and digests where permitted.

## 8. GeoBioBench acceptance and failure dispositions

- **REPRODUCED:** protocol and primary effect direction independently replicated within pre-registered tolerances.
- **NOT_REPRODUCED:** primary effect not replicated under the matched protocol.
- **INCONCLUSIVE:** confidence intervals or assay sensitivity cannot distinguish the relevant effects.
- **PROCESS_ADVANTAGE:** downstream purified-product/resource metrics outperform matched baseline under the preregistered decision rule.
- **NO_PROCESS_ADVANTAGE:** extraction effect exists but process burden erases the advantage.
- **UNQUANTIFIED:** key resource, product, uncertainty, or provenance streams are missing.
- **INVALID:** contamination, calibration failure, protocol breach, or experimental-unit failure invalidates the run.

A failed replication is useful evidence. Do not change endpoints or select a more favorable element after seeing results without declaring a new versioned study.

## 9. What would constitute a genuine breakthrough?

The strongest result would not be a higher isolated fold-change. It would be a reproducible process that:
- works across more than one mineralogical batch;
- produces purified on-spec material;
- requires materially less total imported reagent or energy than a matched abiotic route;
- remains stable over extended reactor operation;
- has a complete elemental and water balance within measurement uncertainty;
- operates with contained biology and a validated contamination response;
- can be repeated by an independent laboratory.

Only then should GeoBio begin to argue that biological processing is a useful industrial unit operation for extraterrestrial resource systems.
