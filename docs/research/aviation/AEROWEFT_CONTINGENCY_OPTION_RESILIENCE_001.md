# AeroWeft — Contingency Option Resilience and Recoverability

**Tracking issue:** #5536  
**Parent stack:** Track T #5362 → Track U #5371 → Track V #5394  
**Status:** synthetic-first, evidence-bound option-set evaluator  
**Claim ceiling:** candidate option admission under a frozen contingency policy; no flight-safety, certification, route-execution or operational-authority claim.

## Research question

When the nominal flight plan becomes unavailable or assumptions fail, does the plan retain a set of separately admissible recovery options across the declared contingency envelope—or has nominal-route optimization consumed the time, energy, maneuverability and airspace flexibility needed to recover?

Recoverability is not nominal feasibility at departure. It is continued option availability under explicitly named failures, changed restrictions, uncertainty and evidence degradation.

## Why this is a distinct capability

NASA's UTM CMSA safety assessment explains that after a UA becomes contingent, future nominal operational-intent volumes may no longer describe its actual path; off-nominal volumes need to be updated so strategic deconfliction reflects the contingent trajectory.

NASA's 2025 risk-aware routing study models loss of command-and-control link and compares contingency routes using last-known traffic interaction risk with previously cleared nominal routes. It reports modeled improvements in safety and efficiency when traffic interaction risk informs routing.

NASA's fmdtools AirspaceLib example demonstrates that mission-specific backup resources and bail-out points matter, and that dynamic threats make replanning more difficult than static obstacles.

These motivate an option-preservation benchmark in addition to route cost, nominal conflict rate and average delay.

## Paradigm shift

Move from “find one nominal route with low delay and energy” to:

**Preserve a bounded portfolio of admissible recovery options under explicit contingency, evidence, common-mode and resource constraints.**

Do not reward the raw number of routes. Two geometrically different options sharing the same failed C2 link, GNSS receiver, landing site, corridor or stale surveillance assumption may represent only one effective recovery choice.

## Ownership and composition

Track W owns:
- the contingency trigger profile;
- fallback-option identity and admission semantics;
- conservative time, energy, occupancy, separation and ground-risk slack;
- explicit dependency/failure-domain inventories;
- a deterministic pairwise-independent option witness;
- adversarial and monotonicity fixtures.

It consumes:
- Tracks M/N/O/P for surveillance, conformance, composition and exact subject/intent/authority receipt binding;
- Track Q for authenticated provenance and trust-policy evidence;
- Track R for conflict-performance evidence;
- Track S for multi-subject scope;
- Track T for model/dataset domain applicability;
- Track U for bitemporal change impact and requalification;
- Track V for experiment selection and validation evidence;
- existing Symtropy reachability, dynamics, physics and trajectory owners.

It does not create a new aircraft dynamics model, PNT/surveillance stack, trust registry, event store, generic lease/currentness service, conflict detector, route optimizer or physics qualification corpus.

## V0 data contract

A contingency case binds:
- aircraft, flight, mission, intent revision/digest and authorization revision/digest;
- model artifact, validation-package and operating-domain digests;
- navigation/time/frame basis;
- trigger class and failed dependency domains;
- effective-time and knowledge-time cutoffs;
- response deadline;
- required energy reserve;
- minimum reachable-occupancy containment lower bound;
- minimum separation-margin lower bound;
- maximum ground-risk upper bound;
- required independent-option count.

Each candidate option binds the same exact subject scope, owner-provided applicability/evidence/authority states, validity end time, receipt IDs and evidence-binding digest, conservative option bounds and complete dependency/failure-domain inventory.

Option inventory has separate effective-time and knowledge-time frontiers. Missing or partial inventory is not evidence that no option exists.

## Conservative gates

For each option, a separately frozen policy checks:
- response deadline minus response-time upper bound;
- available-energy lower bound minus consumption upper bound minus required reserve;
- occupancy-containment lower bound minus minimum;
- separation-margin lower bound minus minimum;
- maximum ground-risk allowance minus ground-risk upper bound.

A negative slack is a definite rejection. Unknown evidence, authority, applicability or dependency inventory is indeterminate. A wrapper or fresh delivery does not upgrade expired or rejected evidence.

## Common-mode diversity

Every option names its critical dependency/failure domains. Any dependency known to have failed in the contingency rejects that option.

A profile may explicitly exempt unavoidable common platform dependencies from the option-diversity calculation. These exemptions never exempt the dependency from failed-domain rejection.

For up to the policy cap of 24 admissible options, the evaluator constructs an exact maximum pairwise-independent set using a deterministic branch-and-bound solver and lexicographic tie-breaking. Above the cap it returns a deterministic greedy witness, marks it non-exact and preserves only the lower-bound claim. Empty dependency sets after common-mode exemptions are never treated as independent.

Consequently:
- a witness meeting the required count can establish COVERED_WITH_REQUIRED_DIVERSITY;
- a complete inventory with multiple admissible options whose exact maximum independent set is smaller than the admissible inventory establishes COVERED_BUT_CORRELATED;
- when every admitted option is independent but the complete inventory contains fewer options than required, V0 returns INSUFFICIENT_INDEPENDENT_OPTIONS. This is a determinate shortage, not a correlation finding or unknown;
- when the greedy witness is too small but a larger set might exist, V0 returns INDETERMINATE rather than overclaim.

This deliberately avoids presenting an NP-hard maximum-independent-set result as an inexpensive heuristic.

## Aggregate statuses

- COVERED_WITH_REQUIRED_DIVERSITY
- COVERED_SINGLE_OPTION
- COVERED_BUT_CORRELATED
- INSUFFICIENT_INDEPENDENT_OPTIONS
- NO_ADMISSIBLE_OPTION
- INDETERMINATE
- REAUTHORIZATION_REQUIRED
- OUT_OF_DOMAIN
- UNSUPPORTED_CONTINGENCY

Every assessment preserves per-option rejection and uncertainty reasons, conservative slack values, failed domains, admissible IDs, the independent-option witness, and inventory completeness. A known failed option cannot erase a different option's uncertainty; unknown inventory cannot become a positive or negative completeness claim.

## Frozen synthetic adversarial corpus

The Rust suite includes:
- positive control with two disjoint dependency sets;
- different route geometries sharing a critical C2 or GNSS dependency;
- known failed dependency and incomplete dependency inventory;
- inadequate energy reserve, response-time deadline, occupancy containment, separation margin and ground-risk upper bound;
- exact threshold equality;
- reauthorization-required, explicitly unauthorized and unknown authority;
- qualified, rejected and unknown evidence;
- evidence expiry;
- authorization revision and model artifact mismatch;
- out-of-domain and unknown applicability;
- unsupported contingency type;
- partial/unknown option inventory and empty complete inventory;
- mixed known failure plus unknown candidate;
- explicit common-platform diversity exemption while preserving failed-domain rejection;
- required diversity greater than the exact maximum or deterministic lower-bound witness;
- option permutation invariance;
- a greedy-order trap where a first-fit witness finds one option but the exact solver finds the two-option independent set;
- options whose only dependencies are exempt common platform domains do not count as independent;
- deterministic lexicographic tie-breaking among equal-size maximum sets where two distinct maximum sets actually exist;
- complete but too-small independent inventory is distinguished from common-mode correlation;
- option counts above the exact-search cap return a non-exact lower-bound witness and abstain when it cannot prove required diversity;
- policy attempts to raise the exact-search cap above 24 are rejected;
- duplicate option, receipt and dependency identities;
- malformed digest;
- unknown dependency set excluded from diversity witness;
- reauthorization gap when a second option is needed.

## Hard gates

Prospective targets:
- zero known-failed dependency accepted;
- zero unauthorized, expired or out-of-domain option credited;
- zero unknown/common-mode dependency counted as pairwise-independent;
- 100% exact agreement with an independent finite option-set oracle for all cases within the frozen exact-search cap;
- zero maximum-cardinality claims when the exact-search cap is exceeded; return a deterministic lower-bound witness and explicit non-exact flag instead;
- zero conservative response/energy/containment/separation/ground-risk gate breaches accepted;
- zero incomplete option inventory interpreted as “no option” or “all options covered”;
- 100% permutation-invariant canonical output;
- zero false maximum-diversity claims: bounded sets expose an exact maximum independent set; larger sets expose a deterministic lower-bound witness with `independent_option_witness_exact=false` and remain indeterminate when the required diversity is not proven;
- report admissible option count, independent witness, failed dependencies, per-option margins and unresolved cases separately.

Scale targets after correctness:
- 10, 100, 1,000 and 10,000 candidate options;
- 10, 100 and 1,000 contingency cases;
- 1/2/3/4-way common-mode combinations;
- p50/p95/p99 runtime, peak memory, witness size, admissible count and unresolved fraction.

Compare against fixed single fallback routes, nearest-safe-site routing, risk-aware C2-loss routing and robust multi-option planning under identical aircraft, failure, evidence and traffic assumptions. Report recoverability coverage and worst-case option loss alongside delay, energy and capacity. Route count alone is not a performance metric.

## Industry/research anchors

- NASA/TM-20240015613: contingent-state and CMSA update of off-nominal operational-intent volumes.
- NASA NTRS 20250006291 (AIAA Aviation Forum 2025): risk-aware routing during C2-link loss using traffic-interaction risk.
- NASA fmdtools AirspaceLib: mission-specific backup resources, bail-out points, battery/propulsion failures and dynamic replanning.
- NASA-STD-7009B / NASA-HDBK-7009B: model credibility and intended-use assurance for the underlying recovery models.
- ASTM F3705-25 / F3442-26: matched-domain DAA comparisons only.

References:
- NASA contingency-management CMSA safety assessment: https://ntrs.nasa.gov/citations/20240015613
- NASA risk-aware contingency routing (2025): https://ntrs.nasa.gov/citations/20250006291
- NASA fmdtools contingency-management example: https://nasa.github.io/fmdtools/examples/airspacelib/contingency_management/demo_overview.html
- NASA-HDBK-7009B (2026-02-03): https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009
- NASA-STD-7009B: https://standards.nasa.gov/standard/NASA/NASA-STD-7009
- ASTM F3705-25: https://store.astm.org/f3705-25.html
- ASTM F3442-26: https://store.astm.org/f3442-26.html

## Claim ceiling

A passing result proves only that supplied synthetic option/evidence declarations satisfy a frozen recoverability policy. It does not prove physical maneuverability, actual aircraft energy, real separation, ground-risk safety, route feasibility in an unmodeled environment, operational authorization, flight safety, certification or permission to execute a contingency.


The current synthetic-first suite has **50 Rust tests**. The exact diversity solver is tested against a greedy-order trap, exempt-only dependencies, a genuine equal-cardinality lexicographic tie, a determinate insufficient-option case, above-cap abstention and policy-cap rejection. All fixtures remain synthetic; the exact solver establishes graph-theoretic dependency diversity under supplied inventories, not physical independence or operational safety.
