# Repair, upgrade and resilient rebuild decision case v0.1

**Parent:** [GRID-RESILIENCE-REBUILD-001 #6278](https://github.com/Luminous-Dynamics/symtropy/issues/6278)  
**Programme:** #5241 Power Fabric benchmark  
**Snapshot:** 2026-11-19

## Purpose

This package defines a two-horizon decision record. It deliberately separates:

1. **Emergency restoration:** restore the declared essential-service floor through an admissible temporary bypass, mobile supply, like-for-like repair, reconfiguration proposal, defer or abstain.
2. **Long-term capital transformation:** evaluate like-for-like rebuild, hardening retrofit, replacement, redundancy, relocation, modularization or distributed alternatives under a declared lifecycle horizon.

The capital analysis is forbidden from delaying an admissible emergency restoration. A capital option is a candidate only and can proceed only after emergency acceptance and a separate authority/engineering approval.

## No duplicate physics or cost engines

This schema references existing owners:
- #6213 execution history/commitment integrity;
- #6273 inspection and evidence acquisition;
- #6237 supplier/factory/port/vessel/resource portfolio;
- #6235 service outcome trajectory;
- #3418 and specialist engineering owners for electrical, protection, network, microgrid and switching admissibility;
- #6214 rights-aware household/organization/community/town/city federation.

The schema does not calculate load flow, fragility, hazard probability, NPV, carbon, or expected losses. It binds model revisions and cost/rights/environment/authority evidence references; specialist owners provide their qualified outputs.

## Two-horizon hard rule

`emergencyPlanMustNotWaitForCapitalReview = true`  
`capitalExecutionRequiresEmergencyAcceptance = true`  
`capitalExecutionRequiresSeparateApproval = true`  
`serviceFloorIsHardConstraint = true`

A quick temporary restoration is not automatically the final asset design. Conversely, a theoretically better long-term design is not a reason to postpone an already admissible emergency service option.

## Scenario and cost provenance

The hazard manifest is a synthetic deterministic scenario set with no assigned probabilities. Do not convert its three enumerated scenarios into a probability distribution or p95/CVaR claim. Capital options bind:
- currency and price-base year;
- lifecycle horizon and discounting convention reference;
- incremental cost evidence reference;
- hazard scenario set;
- lifecycle service trajectory;
- engineering qualification;
- land/rights review;
- environmental review;
- supplier/resource capacity;
- installation window and authority requirement.

Missing costs, fragility, service effects, or hazard frequencies stay unknown. Use robust scenario comparisons until calibrated, applicable probabilities are available.

## Benchmark arms

Compare like-for-like baseline, emergency-only, hardening-first, post-event retrofit, replacement/redundancy, relocation, staged adaptive portfolio, robust/interval policy and a bounded exact oracle for tiny options.

Report emergency first-service time, service floors and service deficit separately from lifecycle service deficit, recurrence exposure, capital cost, embodied material, maintenance and distributional effects. No single composite score is allowed to hide a worst-beneficiary service-floor breach.

## Synthetic fixture

The public fixture is illustrative only. It includes:
- a municipal-feeder transformer with two emergency service options;
- three deterministic hazard scenarios (medium flood, extreme flood, compound heat) with no assigned probabilities;
- like-for-like rebuild, flood-hardening retrofit and redundant-feed capital options;
- explicit service floor, engineering, rights, environmental, supplier, installation and authority references.

No option is selected as technically valid or executed. The benchmark must obtain specialist model/owner receipts before comparing actual admissibility.

## External context

The OECD–World Bank 2026 Quality Infrastructure compendium frames rebuilding as a forward-looking intervention to address structural weaknesses rather than merely restore the pre-event state (DOI 10.1787/6981eda5-en). IEEE PES TR148 (1 October 2026) places resilience within planning, operation and investment. These provide policy/method context, not numerical evidence for this synthetic case.

## Claim boundary

Schema, synthetic scenario manifest and candidate decision record only. No engineering design, hazard probability, load flow, cost estimate, project approval, land access, procurement, construction, energisation, or field resilience improvement is produced.
