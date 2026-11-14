# LEAD-LAG-RSSCT-QUAL-001 — Adsorption Breakthrough and Accelerated Column Qualification v0.1

**Tracking issue:** [#5093](https://github.com/Luminous-Dynamics/symtropy/issues/5093)  
**Filter condition:** [#5084](https://github.com/Luminous-Dynamics/symtropy/issues/5084)  
**Hydraulic cell:** [#5087](https://github.com/Luminous-Dynamics/symtropy/issues/5087)  
**Status:** synthetic F0 arithmetic/state contract only.

## 1. Two complementary innovations

**Lead–lag adsorbers:** a lead bed handles most contaminant mass; a lag bed is a qualified guard. Interstage sampling detects lead-bed breakthrough for a named analyte; final outlet sampling independently validates the guard-treated water. Rotation is a controlled maintenance policy, not an autonomous model action.

**RSSCT:** accelerate media testing using smaller particles and validated similarity assumptions. CD and PD scaling must both be reported until independent pilot data establish which assumptions apply for the media, contaminant and source-water matrix.

## 2. RSSCT equations and limitations

Let SF = particle diameter in the large column divided by particle diameter in the small column. With diffusivity exponent X:
- CD-RSSCT: X=0, EBCT/time ratio = SF^-2.
- PD-RSSCT: X=1, EBCT/time ratio = SF^-1.

The ratios are used only if the sorption isotherm and mass-transfer similarity assumptions are appropriate. The 2026 scale-up review says RSSCT may not capture fouling or long-term aging and recommends pilot confirmation. Recent PFAS work finds the CD/PD assumptions can be inconclusive or violated for some media/contaminant/matrix combinations.

Example: SF=5 gives CD duration ratio 1/25 and PD ratio 1/5. For a two-year illustrative full-scale target, the simple durations are ~29.2 days and ~146 days. These are arithmetic examples, not an established lifetime prediction.

References:
- 2026 scale-up review: https://doi.org/10.1021/acs.est.6c07585
- PFAS RSSCT/pilot validation: https://pmc.ncbi.nlm.nih.gov/articles/PMC11706541/
- Adsorber pilot scaling overview: https://ascelibrary.com/doi/full/10.1061/JOEEDU.EEENG-8317

## 3. Lead–lag evidence semantics

Interstage sample applies to the lead-bed outlet/lag-bed inlet. Final outlet sample applies to post-lag water. They are distinct sample events and cannot be substituted for each other.

Only accepted quantified samples bound to the exact analyte claim may be compared against the registered C/C0 decision threshold. Below-LOD/LOQ is censored; rejected QC is not evidence; missing sample is not evidence of no breakthrough. A generic TOC/UV254/conductivity proxy is not treated as target-analyte measurement unless separately validated.

A lead-bed breakthrough with a verified guard identity may create a **review rotation advisory**, but never authorizes valve actuation or final water release. An unverified guard identity or broken service history forces abstention.

## 4. F0 cases

Six synthetic scaling cases cover scale factor 2, 5, and 10 under CD and PD assumptions. Five synthetic lead–lag cases cover no sample, accepted interstage breakthrough with no final sample, interstage breakthrough with accepted below-threshold final sample, censored interstage sample, and loss of guard identity/service continuity.

## 5. A1–A5 physical validation

A1 bench: exact media batch, particle size, column diameter/bed depth, mass, porosity, EBCT, flow/superficial velocity, feed matrix, pressure drop and sample plan.
A2 RSSCT: parallel CD and PD columns with replicates and pre-registered scaling assumptions.
A3 pilot comparator: match media batch and source-water matrix; compare breakthrough curves at 0.05, 0.1, 0.5 and the claim-specific threshold.
A4 lead–lag: interstage/final sample chronology, guard remaining capacity and lead-bed change procedure.
A5 lifecycle: replacement, regeneration, spent-media handling, downtime, labour, water and energy.

Report bed volumes to each threshold, C/C0 curves, mass balance, pressure drop, guard reserve, predicted-vs-pilot breakthrough error and lifecycle cost per volume of profile-conforming water.

## 6. Nonclaims

No adsorbent, filter, RSSCT scale-up, contaminant removal, breakthrough protection, pilot performance, bed rotation policy, potability, certification, media life or autonomous operation is qualified by this synthetic F0 contract.
