# Construction Cost per Accepted Output — Cost Ledger Contract V0.1

**Issue:** [#3631 — lifecycle cost per independently accepted work unit](https://github.com/Luminous-Dynamics/symtropy/issues/3631)  
**Generic output contract:** [#3605](https://github.com/Luminous-Dynamics/symtropy/issues/3605)  
**Schema:** `schema/construction-cost-ledger-v0.1.schema.json`  
**Layout cost profile:** `profile/layout-cost-profile-v0.1.json`  
**Status:** proposed economic evidence contract; no robot acquisition or project ROI has been evaluated.

## Purpose

The cost denominator is finally accepted, independently evidenced, task-specific output. This package defines how costs, prices, time allocations and capital assumptions are recorded so they can be reconciled to the accepted-output ledger.

It does not produce a universal robot ROI. It does not recommend procurement, finance assets, set discount rates or certify savings. Every cost claim must state its currency, price date, tax basis, source class, allocation rule and denominator.

## Cost boundary

The full lifecycle boundary may include:

1. **Acquisition and finance:** purchase, options, shipping/import, installation, commissioning, tariff/tax, lease, interest and working capital.
2. **Capital ownership:** useful life, depreciation, replacement, residual value, capital cost and utilization.
3. **Fixed service:** licence/software, connectivity, remote operations, cybersecurity, support contract, insurance, training and facilities.
4. **Operations:** labour, telehandler/forklift, fuel/energy, consumables, materials, wear parts, tooling, calibration, survey, logistics and replenishment.
5. **Failure/recovery:** maintenance, diagnosis, repair, spares, service travel, idle labour/equipment, rejected output, rework, replacement and schedule disruption.
6. **End-of-life:** decommissioning, disposal, asset resale, batteries/material handling and workflow transition.

Record costs as measured/contractual/quoted/allocated/estimated/assumed. A vendor list price or public claim is not a paid invoice. A cost estimate is not an observed saving.

## Cost-entry contract

Every entry binds to:
- the accepted-output record and exact task profile;
- project/site/work-package/robot serial/configuration and cost period;
- category, amount, currency and currency minor-unit exponent;
- price date, tax treatment and nominal/real basis;
- source class and evidence reference;
- measurement/contractual/estimated basis;
- allocation method and versioned allocation-policy identity;
- direct resource/work-unit/time-entry bindings where applicable;
- whether rates already include labour, overhead, service or other included categories.

### Currency and precision

A ledger uses one reporting currency. Amounts are stored as integer minor units with the ISO 4217 currency code and declared minor-unit exponent, avoiding binary floating-point arithmetic for money. Cross-currency comparison requires a separate dated FX conversion record or harmonisation process with source and rate evidence; never silently convert or use a current spot rate for historical costs. Entries also declare `cost` versus `credit`; residual/salvage value is a credit, never a positive cost. The v0.1 schema restricts credit-direction entries to the explicit residual-value credit category.

### Allocation and double counting

Shared robot capital, supervisors, telehandler, mobilisation, site infrastructure and maintenance contracts need explicit allocation methods. Record whether cost is direct, metered, time-based, operating-hour-based, capacity-reservation-based, accepted-output-based or otherwise allocated.

A loaded labour rate that includes overhead must not be added to a separate overhead charge for the same resource/period. Bind the exact time-entry references or disclose that the rate is standalone. Cost categories included in a rate must be declared. If the allocation is estimated or assumed, preserve the assumption and run sensitivity analysis.

No allocation policy is universally correct. The same source cost can yield different per-unit costs under different utilisation/asset-sharing assumptions; publish the selected policy and alternatives.

## Derived outputs

The cost report should keep separate:
- total cost by category and source class;
- capital/fixed/variable/operating/rework/service cost per finally accepted unit;
- robot-only operating cost per robot-hour;
- direct site cost per direct crew-hour;
- total workflow cost per accepted unit;
- incremental cost versus matched current practice;
- break-even annual accepted output and utilisation;
- low/base/high and evidence-supported probabilistic sensitivity;
- residual value, financing, discount/inflation and useful-life assumptions.

Do not calculate a rate if the denominator is zero or unresolved. Do not count rejected/uninspected output as accepted quantity. Do not use a single payback period without showing financing, asset life, utilisation, depreciation, service, residual and all workflow costs.

## Accepted output and safety separation

Cost entries consume the accepted-output disposition and resource-time evidence from #3605. Productivity or cost reduction cannot change the safety verdict. Safety holds, inspections and required maintenance are not automatically avoidable waste.

The economic report must disclose costs required to meet the independently qualified ODD and safety process. A cheaper configuration that fails its safety requirements is not a valid economic winner.

## Industry examples are not price-performance conclusions

FBR's official Hadrian page currently lists a base price of A$7.8 million, subject to final specifications/options. This is a vendor-published price indication, not a transaction price or a fully loaded TCO. The Fortescue/ARENA Cloudbreak report states that a direct cost comparison was not practically determinable at trial scale and with differing contractual/methodological boundaries. These are reasons to formalize cost evidence—not invitations to invent ROI.

References:
- Hadrian official specifications/price indication: https://www.fbr.com.au/view/hadrian
- Fortescue/ARENA Cloudbreak trial report: https://arena.gov.au/assets/2026/03/Fortescue-Pilbara-Solar-Innovation-Hub-Built-Robotics-Trial-Report.pdf
- Brosque & Fischer (2022), ten robots / twelve projects / eleven contractors: https://link.springer.com/article/10.1007/s41693-022-00072-5

## Initial reporting gates

- 100% of cost entries bind currency, exponent, price date, source class, category and allocation method.
- 100% of compared costs bind the same accepted-output profile and work-package scope.
- 100% of shared/allocated cost entries disclose allocation policy and any included labour/overhead categories.
- 100% of cost reports reconcile to source cost entries and report unallocated unknowns.
- Zero cost credit for unresolved, uninspected or rejected output.
- No causal savings claim without a matched baseline or transparent counterfactual.
- No ROI league table across unlike functional units.

## Ownership

- #3631 owns lifecycle cost methodology and economic claim reporting.
- #3605 owns accepted-output units and resource-time accounting.
- #3591 owns cross-laboratory metrology/reproducibility.
- #3523/#3524, #3626/#3627 and #3629/#3630 own task-specific output profiles.
- #3531 and Symthaea #4918 own claim admissibility and dependency lineage.
- Construction safety owners retain independent, non-compensable safety qualification.

This package is a versioned cost-evidence contract, not a procurement decision engine, accounting system, financial advice or universal ROI calculator.
