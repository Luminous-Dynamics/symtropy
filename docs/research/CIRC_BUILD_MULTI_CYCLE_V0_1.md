# CIRC-BUILD-002 — multi-cycle degradation and maintenance-policy benchmark v0.1

**Status:** standalone synthetic Rust reference benchmark  
**Owner:** Symtropy  
**Composes:** CIRC-BUILD-001 lifecycle Pareto work, Mycelix provenance, Symthaea decision-value proposals, Sol-Atlas spatial context  
**Claim ceiling:** synthetic contract conformance only

## Paradigm shift

A one-time end-of-life mass balance does not describe a lifecycle. Decisions interact over time: condition degrades with use; repair restores condition while consuming parts; remanufacture renews capability while retaining only a declared share of original lineage; replacement adds virgin material and resets condition but ends the original asset's service contribution.

Evaluate the whole service horizon. Every plan attempts the same service quantity under the same condition thresholds, repair limits, evidence requirements, and system boundaries. No incomplete plan is allowed onto the Pareto frontier. No aggregate circularity score compensates for a failed safety, evidence, function, or inspection gate.

## Frozen reference model

- Synthetic asset mass: 1,000 g.
- Required horizon: three cycles of 10,000 qualified operating hours each.
- Condition and lineage state are bounded integer basis points (0–10,000).
- Initial condition and initial asset lineage share: 10,000 bp.
- Condition degrades by 2,500 bp after each serviced cycle.
- Minimum pre-service condition: 6,000 bp.
- Repair restores the profile's declared condition and consumes one repair allowance.
- Remanufacture restores condition to 10,000 bp, resets the repair counter, and caps original-asset lineage at its declared retained share (50% in the fixture). Lineage is monotonic non-increasing: remanufacturing a replacement asset cannot restore credit to the retired original.
- Replacement restores condition to 10,000 bp, resets repair allowance, and assigns zero lineage share to the initial asset.

**Lineage-weighted service hours** are explicitly a synthetic bookkeeping proxy for the amount of service attributed to the initial asset. They are not a direct physical measurement or universal circularity metric.

## Per-cycle state transition

1. Resolve an action profile and its evidence reference.
2. Validate its source digest, status, claim/scope, coverage, safety/function/inspection gates.
3. Validate mass fates: each retired gram is assigned to reuse, material recycling, energy recovery, or residual; the sum must equal retired mass. A replacement action must account for exactly the declared asset mass.
4. Apply the action's condition/lineage effects and enforce repair count.
5. If pre-service condition is below the threshold, stop and do not credit service for that cycle.
6. Accumulate lifecycle metrics, deliver one service block, add lineage-weighted service, degrade condition, and append the state trace.
7. Accept only plans that deliver the full horizon.

## Objective vector

For complete plans, minimize total lifecycle cost, virgin input, energy, GHG impact, water, and residual waste; maximize lineage-weighted service hours. Total delivered qualified service is equal across eligible candidates. Report all dimensions and the non-dominated frontier without scalar weights.

## Proposed benchmark targets

These are targets, not achieved results:

- 100% agreement with the independent transition oracle over the frozen plan corpus.
- 100% agreement between production frontier selection and an independently structured exhaustive objective-vector oracle.
- Zero incomplete-horizon plans in the eligible set.
- 100% rejection of the frozen unknown/revoked/partial/out-of-scope evidence, failed inspection, repair-limit, mass-imbalance, invalid-plan-length, and unknown-action controls.
- Exact fate conservation for every action, with no energy-recovery/recycling double count.
- Deterministic normalized receipts bound to manifest bytes, source revision, and tool version.
- Report plan denominator, condition trajectory, repairs consumed, delivered service, lineage-weighted service, resource totals, fate totals, rejection reasons, fixture digest, source revision, and tool version.

## Extension ladder

**L0 — Single-cycle evidence-gated Pareto:** [#1856](https://github.com/Luminous-Dynamics/symtropy/issues/1856), draft [PR #1864](https://github.com/Luminous-Dynamics/symtropy/pull/1864).  
**L1 — Multi-cycle horizon and degradation:** this benchmark, [#1878](https://github.com/Luminous-Dynamics/symtropy/issues/1878).  
**L2 — Remaining-life uncertainty and inspection value:** compose Symthaea's SE-FOSS-003F proposals in [symthaea#6203](https://github.com/Luminous-Dynamics/symthaea/issues/6203) after prerequisites qualify. A ranked proposal is neither evidence nor actuation authority.  
**L3 — Multi-organization provenance:** Mycelix [#5149](https://github.com/Luminous-Dynamics/mycelix/issues/5149) binds approvals to exact asset, source lots, inspections, and report identity.  
**L4 — Spatial recovery feasibility:** Sol-Atlas [#142](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/142) contributes source-grounded facility locations, capability, geographic coverage, and transport context.  
**L5 — Independently measured pilot:** only after material characterization, calibration, approved procedures, and a function-matched conventional baseline.

## Research anchors

- ISO 59020:2024 — circularity measurement using declared boundaries and reproducible indicators. https://www.iso.org/standard/80650.html
- ISO/WD 59020 Edition 2 — working draft under development; pin the edition used. https://www.iso.org/standard/94166.html
- Regulation (EU) 2024/1781 — product-specific ecodesign and end-of-life information framework; applicability depends on product-specific measures. https://eur-lex.europa.eu/eli/reg/2024/1781/oj
- Costa, Hoolahan & Charef (2025), materials passports and reuse in construction. https://doi.org/10.1038/s44296-025-00079-3

These anchors do not define the synthetic numbers or qualify the state-transition model. No real degradation, LCA, product certification, safety, or environmental improvement is claimed.
