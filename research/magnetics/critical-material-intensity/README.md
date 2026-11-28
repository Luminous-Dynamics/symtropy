# Critical-element intensity per accepted service

**Programme:** MAG-CRITICAL-ELEMENT-INTENSITY-001  
**Issue:** #7912  
**Revision:** 0.1-draft

## Purpose

Compare complete magnetic-system architectures by the measured mass of each virgin element consumed per MWh of accepted useful shaft service. The output is a vector, not a single score: virgin Nd, Pr, Dy, Tb, Co, Cu and every other element preregistered from the complete bill of materials.

This avoids treating magnet volume as a proxy for rare-earth use. Two magnets of equal mass can have different element compositions; a lower magnet mass can hide a higher Dy/Tb fraction or material substitution elsewhere in the system.

## Evidence contract

The benchmark freezes application, target function, duty profile, system boundary, inverter/control, cooling, measurement setup, acceptance policy, service horizon and the complete element-inventory/BOM scope.

Every architecture trial requires M4 physical component acceptance; physical useful shaft energy per mission and an integer count of completed missions; separate physical durability evidence; exact material lot IDs and measured mass; per-element mass-fraction intervals for every material lot; laboratory and method-scope, method, calibration and uncertainty receipts; a complete preregistered element set; and an optimization receipt with frozen search space, objective/constraints, budget tier and restarts.

A modelled lifetime projection cannot support an observed lifetime-service claim. Validated projections belong in a separate scenario analysis and are not admitted by the v0.1 decision evaluator.

## Origin accounting

The evaluator distinguishes Virgin, ExternalSecondary, InternalRecycled and DirectReuse. For each element it reports the four origin masses separately, their total embodied mass, and corresponding g/MWh intensities. No end-of-life recycling credit is subtracted from virgin input in v0.1. This avoids silently combining avoided-burden or substitution credits with virgin-input reduction.

A non-detect must be represented by a measured interval bounded by the method's detection/quantification limit, not omitted or silently treated as zero. The full element set must come from the complete BOM and inventory-scope record; the evaluator binds that scope digest but does not authenticate the BOM issuer.

## Decision rule

An AM/hybrid candidate can be a CriticalMaterialAdvantage only if candidate and baselines share the exact application, duty, inverter/control, cooling, system, measurement and service-horizon scope; both have M4 physical component acceptance; accepted service has physical shaft-energy and durability receipts rather than projections; optimization evidence is budget-tier matched and frozen before unseal; all required element assays and material records are valid; candidate is robustly non-inferior on accepted service and every preregistered element-specific virgin intensity; and it strictly improves at least one element-specific intensity against every required baseline.

No weighted criticality scalar is emitted. Cost, CO2, LCA, supply concentration, geopolitical risk and recyclability remain separate dimensions with their own boundaries and evidence.

## Competitive baselines

Select the strongest industrially relevant comparators for the application: optimized sintered-NdFeB IPM, bonded PM where relevant, PM-assisted SynRM, wound-field synchronous, induction, SynRM/reluctance control, and the AM/hybrid candidate.

Devito et al. (2026), DOI 10.3390/en19184316, summarizes application-specific findings: one 60 kW light-vehicle comparison reported IPM/WFSM/SynRM drive-cycle energy of 93.70/94.24/94.86 Wh/km (about 1.3% spread at its stated boundary), while one PM-assisted SynRM retrofit study reported 26% lower rare-earth magnet volume without performance loss. The review also summarizes a hypercar FEA/design case with about 45% magnet-volume reduction and a small drive-cycle penalty, and a heavy-duty case with a different WFSM/IPM ranking. These are not matched universal SOTA figures; preserve original application, evidence class, optimizer and system boundary.

## Proposed programme targets

These are aspirational gates, not measured results:
- **Initial challenge:** at least 10% lower virgin Nd+Pr g/MWh than the optimized applicable PM baseline, while Dy/Tb/Co and every other preregistered element remain non-inferior.
- **Stretch challenge:** at least 25% lower virgin rare-earth vector consumption per MWh, with no hidden element substitution or deterioration in accepted service, reliability or system energy.
- **Evidence target:** zero false-safe results in sealed negative controls; 100% element/BOM scope coverage; 100% material-lot and assay-receipt joins; no omitted/duplicated lots; no projected service substituted for observed durability.

The targets are not weighted aggregates. Nd, Pr, Dy, Tb, Co and other elements remain separate rows/vectors.

## Integration

- #7802 architecture benchmark supplies matched system-level function and non-inferiority.
- #7887 duty-cycle evidence supplies ordered mission profile and accepted shaft-energy boundary.
- #7791 recovery passport supplies feedstock and recovered-output genealogy.
- #7870 circular/second-life rotor work supplies teardown and post-service evidence.
- #7776 manufacturing cohort evaluator supplies all initiated builds, scrap, rework and accepted-part denominator.
- #7792 derives evidence maturity.
- #7911 qualifies recovered feedstock routes.

## Validation ladder

E0 schema, identities, complete element scope, no-defaults and sealed negative controls.  
E1 synthetic known-answer mass-fraction × material-mass × service arithmetic, interval uncertainty and duplicate/no-double-counting attacks.  
E2 measured element assays and process material balance.  
E3 matched M4 components with complete mission and element inventory.  
E4 repeated builds/lots, durability, replacements and qualified second-life/recovery.  
E5 independent external replication and separately audited LCA/TEA.

## Claim ceiling

The initial fixture uses SyntheticControl evidence for acceptance, service, durability, material mass and element assays; it must remain Indeterminate. Rust test success validates arithmetic and decision semantics only. No physical material reduction, product lifetime, commercial savings, CO2, supply-risk or commercial advantage is claimed.
