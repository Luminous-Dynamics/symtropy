# Grid service risk certificate qualifier v0.1

**Owner:** Symtropy #6317
**Independent reference:** `reference/interval_simplex_reference.py` exhaustively enumerates all integer-basis-point probability vectors in the three-branch fixture (not the Rust algorithm).  
**Programme:** #5241 Power Fabric benchmark  
**Upstream policy:** Symthaea #12436 / PR #12441, adapted in #6295 / PR #6300  
**Service owner:** Symtropy #6235  
**Beneficiary scope owner:** Symtropy #6214

## Purpose

This package consumes independently accepted branch-level service summaries. It qualifies hard beneficiary minimum-service floors and calculates the worst-case expected unserved quantity for each service metric under a finite interval-simplex ambiguity set.

It does not calculate any service trajectory, probability posterior, power flow, water hydraulics, telecom capacity, policy tree or restoration schedule.

## Three separately pinned input artifacts

1. **Policy certificate reference:** a digest of the upstream finite policy tree. The probability point estimates and branch identities must align with it.
2. **Probability envelope:** branch point probabilities and lower/upper probability bounds, with a separately pinned ambiguity-set/source reference.
3. **Service outcome manifest:** branch-specific, independently accepted service fractions and native-unit unserved quantities. The same metric IDs, cohort, denominator, unit, horizon and minimum floor must appear in every branch.
4. **Risk-limit manifest:** separately pinned per-metric native-unit risk budgets and explicit authority references.

SHA-256 binds each supplied payload; it does not authenticate who issued the payload. Signed upstream receipts or a trusted immutable registry are still required for source authentication.

## Exact interval-simplex worst case

For metric losses Lᵢ and branch probabilities pᵢ constrained by lower/upper bounds:

- initialize pᵢ at each lower bound;
- compute remaining probability mass to 10,000 basis points;
- allocate that mass to branches in descending Lᵢ order, without exceeding their upper bounds;
- calculate the resulting expected unserved quantity.

This greedy allocation is exact for the finite probability-box/simplex set and a linear expected-loss objective. It is not a fitted distribution or a calibrated tail forecast.

## Synthetic case outputs

Point branch weights are 3,000 / 1,400 / 5,600 bp. Declared intervals are:
- replacement after first positive: 2,500–3,500 bp;
- replacement after first negative/second positive: 1,000–2,000 bp;
- repair after two negatives: 5,000–6,500 bp.

The interval simplex is feasible. Its maximum expected deficit allocates 2,500 / 1,000 / 6,500 bp to the three branches.

| Service metric | Unit | Point expected | Robust expected | Budget |
|---|---|---:|---:|---:|
| Hospital critical electricity | MWh | 4.99 | 5.60 | 6.0 |
| Least-served neighbourhood electricity | MWh | 6.62 | 7.30 | 8.0 |
| Community potable water | m³ | 117.8 | 134.5 | 150.0 |
| Rural household telecommunications | customer-hours | 58.34 | 66.85 | 75.0 |
| Metropolitan emergency electricity | MWh | 6.18 | 6.95 | 7.5 |

All service-floor values are synthetic. The fixture deliberately uses five beneficiary scopes: organization, community, town/municipality, household/site and city/metropolitan. Its floor controls are separate from risk-budget controls.

## Hard-floor and uncertainty semantics

- Every accepted lower service-fraction bound must meet its cohort's minimum floor on every in-scope branch.
- Missing, pending, rejected or unknown service evidence produces INCONCLUSIVE; it is never treated as zero deficit or full availability. Unaccepted service values are excluded from floor-breach assertions and from robust expected-risk calculations.
- The probability point estimates must lie inside the interval bounds.
- Point probabilities must sum to 10,000; interval lower sums must be <=10,000 and interval upper sums >=10,000.
- Every branch must carry the exact same set of service metrics.
- A metric cannot change unit, service type, cohort, denominator, horizon or floor between branches.
- Robust expected unserved quantity is compared only with a risk budget for the same metric and unit.
- Electricity, water and telecommunications are never summed into one scalar.

## External comparison

LARA (Warrier et al., Findings EACL 2026, DOI 10.18653/v1/2026.findings-eacl.321) uses a constrained LLM action catalogue and physics-aware simulation to coordinate repair crews, mobile power sources and switching. Its reported IEEE 13/33-node results are a required planning baseline. The open implementation uses a Python/OpenRouter stack and its scenario-parser script includes eval() on parsed archive text; benchmark ingestion should isolate that parser and use a safe data-only adapter.

Lin & Xu (Applied Sciences, 26 October 2026, DOI 10.3390/app162110584) formulate a distributionally robust whole-process black-start/reconfiguration/load-restoration model on IEEE 118-bus, with AC verification and three out-of-sample years. This is a required physical/robustness comparator; our service-risk qualifier does not replace AC or transient verification.

IEEE PES TR143 (June 2026) provides a multi-context power-system resilience metric-selection framework; TR148 (October 2026) calls for risk-based planning and resilience integrated across planning, operations, investment and supply chains. NERC's 2026 State of Reliability and the Zpryme/S&C 2026 survey provide additional industry context, with their reporting scopes and sponsorship disclosed.

## Claim boundary

Synthetic branch-level service-risk qualification only. No service model, operational forecast, risk calibration, power-flow feasibility, dispatch, field measurement or resilience improvement is produced.
