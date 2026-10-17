# CIRC-BUILD-001 — lifecycle intervention and Pareto benchmark v0.1

**Owner:** Symtropy  
**Consumers:** Symthaea (hypothesis/experiment proposals), Mycelix (provenance and accountable decisions), Sol-Atlas (spatial context)  
**Status:** standalone synthetic reference implementation  
**Claim ceiling:** synthetic contract conformance only

## Paradigm shift

Do not optimize a recycled-content percentage or a universal green score. Preserve required function first, enforce safety/quality/evidence as non-compensatory gates, and compare feasible intervention packages with a Pareto frontier.

A kilogram of qualified reused component, a kilogram of downcycled material, and a unit of energy recovery are different outcomes. The benchmark reports final material fates separately and refuses to count energy recovery as material recycling. It also refuses to compare options that deliver different functional units.

## Frozen first scope

A synthetic modular environmental sensor/logger that must deliver **one environmental sensor module delivering 10,000 qualified operating hours**. The fixture contains five scenarios:
- baseline trade-offs between low-cost repair and low-virgin-input remanufacture, plus a dominated option;
- reuse with unknown inspection/evidence against verified repair;
- revoked and contamination-failed recycling candidates against a replacement-plus-energy-recovery package;
- an intentionally double-counted material balance and an option with missing energy data;
- a functional-unit mismatch.

No fixture quantity is a physical measurement, supplier declaration, LCA result, or real-world recommendation.

## Model and deterministic disposition

An option is rankable only if:
1. The referenced evidence's exact payload SHA-256 matches, evidence status is valid, the claim is the lifecycle-eligibility claim, the scope matches, and coverage is complete.
2. Safety, functional delivery, and post-intervention inspection all pass.
3. The option delivers the scenario's exact functional unit.
4. Required lifecycle metrics are present, value retention is within its declared range, and the mass-fate ledger conserves the declared input exactly.
5. Reported residual waste matches the residual mass flow.

Fail-closed states are distinct: eligible, hard-gate rejection, rejected evidence, invalid digest, out-of-scope, incompatible functional unit, rejected mass balance, incomplete coverage, unknown, and invalid metrics.

The mass fate ledger partitions the prior asset into mutually exclusive bins: kept in service, reused component, recycled material, energy recovery, residual. A single mass unit may appear in one final-fate bin only. Replacement embodied impacts are represented in the lifecycle metrics for the same functional service; they are not part of the prior-asset fate partition.

## Objective vector

Minimize each of: lifecycle cost per functional unit, virgin material input, energy, GHG impact, water, and residual waste. Maximize declared material/function value retained (basis points; 0–10,000).

This intentionally produces a set of non-dominated choices instead of pretending the metrics have universal exchange rates. Safety and evidence gates are not objectives and cannot be traded away.

## Benchmarks

Targets, not measured outcomes:
- 100% expected-disposition agreement on the frozen options.
- 100% agreement between production Pareto selection and independent normalized-vector exhaustive pair scanning.
- Zero eligible candidates with failed safety/function/inspection gates, stale/revoked/mismatched evidence, incomplete coverage, invalid mass balance, or missing required metrics.
- Zero mass double counting in the frozen corpus; all prior-asset mass is conserved to exclusive fates.
- Zero silent imputation of missing metrics to zero.
- Deterministic, sorted reports bound to exact fixture-byte SHA-256, source revision, and tool version.
- A lower-cost/higher-virgin option and a higher-cost/lower-virgin option remain separately non-dominated when each wins at least one objective.
- No scalar readiness score; output denominator, hard-gate reasons, all metrics and each Pareto result.

Later real-world targets must be established empirically: first-pass repair/reuse yield, retest failure, qualified service delivered, virgin input avoided, processing/transport energy, GHG, residual waste, cost per accepted functional unit, quality after reuse, and uncertainty, all against a function-matched frozen baseline.

## Extension ladder

**L0** synthetic eligibility + mass-balance contract (this implementation).  
**L1** multi-cycle degradation, repair/retest history, component compatibility, and remaining-life uncertainty.  
**L2** versioned material/process-flow composition with declared coproduct-allocation rules and bounded uncertainty.  
**L3** spatial recovery matching via Sol-Atlas source coverage, distance/transport, facility capability, and supply/demand windows.  
**L4** multi-organization passport / approval receipts via Mycelix, including supplier claims vs independently verified data, correction/revocation, scope and role/authority.  
**L5** Symthaea decision-value experiments to choose inspections that can most efficiently change a candidate's disposition or the Pareto frontier.  
**L6** measured and independently reviewed physical pilot; only this stage can claim observed real-world yield, cost, energy, or environmental performance.

## Research and scope notes

- ISO 59020:2024 specifies a framework for circularity measurement and assessment with defined boundaries and indicators. https://www.iso.org/standard/80650.html
- EU Regulation 2024/1781 creates a product-specific ecodesign/information framework; actual product duties depend on relevant delegated acts. https://eur-lex.europa.eu/eli/reg/2024/1781/eng
- A 2025 materials-passport perspective recommends prioritizing retention and reuse where viable, pre-redevelopment audits, deconstruction plans, interoperable passports, and lifecycle updates. https://doi.org/10.1038/s44296-025-00079-3
- A 2025 review finds that materials-passport methodologies and scope are heterogeneous and that interoperability/data scarcity remain barriers. https://doi.org/10.3390/su17125661

This is not a universal LCA calculator, materials-certification system, GIS, ledger, professional-engineering approval, procurement agent, or physical controller. Do not promote a synthetic PASS to a real product or environmental claim.