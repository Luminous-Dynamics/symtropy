# MUS-OPT-001 — Instrument architecture manufacturability and lifecycle benchmark v0.1

Status: proposed synthetic matrix only. It contains no supplier quotations, physical builds, manufacturing-yield measurements, repair studies, cost estimates or qualified performance evidence.

Parent: https://github.com/Luminous-Dynamics/symtropy/issues/1994
Cross-project design and measurement contract: https://github.com/Luminous-Dynamics/symthaea/issues/8036
Symthaea draft benchmark/audit: https://github.com/Luminous-Dynamics/symthaea/pull/8037
Mycelix hardware release authority: https://github.com/Luminous-Dynamics/mycelix/issues/2392

## Paradigm shift

Compare instrument architectures by **lifetime capability and robust manufacturability**, not the cheapest BOM quote, a simulation score, or an opaque repairability rating. A candidate must satisfy hard constraints before Pareto ranking. Unknown evidence remains unknown and cannot be turned into zero cost or successful qualification.

This matrix is a domain input/corpus, not a second optimizer. Reuse existing Pareto, uncertainty, repair-task and hardware-release owners after confirming the correct owning interfaces. Symthaea owns family-specific physics/design predictions and observation interpretation; Symtropy owns the cross-candidate, manufacturing-route and whole-life trade-off; Mycelix's existing release manifest retains ownership of product/release identity and artifact provenance.

## Synthetic design space

Three architecture archetypes:
1. acoustic resonator with replaceable sensor/pickup;
2. low-voltage electronic instrument/controller;
3. modular electro-acoustic hybrid.

Three proposed route archetypes:
1. hand-workshop assembly;
2. maker-scale digital fabrication plus commodity electronics;
3. qualified contract manufacturing.

These are candidate comparison categories, not claims that every route is technically or commercially appropriate for every architecture. The nine baseline cells are present exactly once to freeze the experimental design. The numeric economic/production inputs remain null.

Four qualitative stressors are included: critical part unavailable, FX/lead-time disturbance, tighter tolerance requirement, and failed build/rework. They deliberately contain no assumed numeric multiplier or local market forecast. Eight negative controls test unknown cost as zero, unsupported substitutions, simulation-to-measurement promotion, omitted failed builds, repair without post-test, ranking ineligible candidates, unresolved asset rights, and stale revision evidence.

## Objective contract

The corpus names these separate objectives and units/directions:
- BOM cost: currency per unit; minimize; ISO 4217 currency code and dated source required once resolved.
- Assembly time: minutes per unit; minimize.
- Calibration time: minutes per unit; minimize.
- First-pass yield: fraction 0–1; maximize.
- Material waste: grams per unit; minimize.
- Energy: watt-hours per unit; minimize.
- Median and p90 repair time: minutes per repair; minimize.
- Critical-part lead time: calendar days; minimize.

Every objective record retains value, unit/direction, evidence status, source reference, as-of date, and absolute uncertainty when available. Under `unresolved`, value/source/date/uncertainty must remain null. A quoted BOM amount is not an invoiced cost; a supplier web listing is not stock confirmation; a simulated yield is not measured process yield.

No Pareto frontier is produced from this matrix. Baselines are `UNRANKED_MISSING_EVIDENCE`; stress scenarios are `BLOCKED_UNVERIFIED`; negative controls carry an expected rejection/block disposition.

## Proposed benchmark gates

**Stage 0 — corpus integrity**
- Exactly nine distinct baseline cells covering 3 architectures × 3 route categories once each.
- No duplicate scenario IDs; no objective key/units/directions omitted.
- 100% named negative controls must reject the unsafe claim, block ranking, or block release.
- Zero unresolved objective promoted to zero or treated as a successful result.
- Deterministic audit report for byte-identical input and same tool version.

**Stage 1 — source-qualified scenario**
- Each numeric cost binds currency, unit, quote/source identity, geography, issue date/expiry, volume assumptions, lead time, MOQ and approval status.
- Include tooling/capex, setup, labor, yield/scrap/rework, qualification, warranty, service inventory and end-of-life, not only component prices.
- Every route's feasibility has evidence and documented prerequisites; unknown legality, safety or process capability blocks the route.

**Stage 2 — bench and workshop pilot**
- Target at least three independently assembled units and, when feasible, three builders, with all failures/rework reported.
- Record at least ten repeated trials per condition during the method pilot, then choose confirmatory sample sizes from measured variance and preregistered precision/power goals.
- Report median/p90 and dispersion for assembly/calibration/repair time, and between-unit/builder variance for measured performance.
- Do not count faster repair if post-repair function fails or the trial skips required checks.

**Stage 3 — robust Pareto comparison**
- Filter infeasible candidates before ranking.
- Report distinct trade-offs for cost, performance, assembly/calibration burden, repair time/downtime, material/energy and supply risk.
- Run sensitivity/scenario analysis on genuinely sourced uncertain inputs. Label modeled stressors as modeled; do not pass off a synthetic multiplier as a local statistic.
- Require a complete denominator covering failed builds, rework, repairs and missing observations.

## Evidence and source policy

Use dated, rights-compatible supplier quotes, purchase records, machine/tool availability, observed assembly and repair trials, calibrated physical measurements, and exact candidate/build identities. Maintain separate columns for quoted, purchased, derived, simulated and physically observed evidence. The same metric can contain a measured value and a separate derived estimate, but they must not be silently merged.

## Nonclaims

This draft does not establish cost, availability, local manufacturability, product safety, repair superiority, market demand, yield, acoustic quality, musical preference or commercial viability. It freezes a testable design space and the rules for later measurements.
