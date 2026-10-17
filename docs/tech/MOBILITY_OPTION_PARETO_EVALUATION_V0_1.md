# Regional Mobility Bundle Evaluation with Hard Gates and Pareto Evidence v0.1

**Status:** synthetic decision-evaluator contract and reference fixture. It does not rank real products or establish safety, market availability, affordability, or environmental impact.

## Paradigm shift: stop asking for a universal best vehicle

The unit of design is the **mobility service** required by a declared user and mission, not necessarily a vehicle. Candidate bundles may include a private vehicle with charging and service access, a repairable used vehicle with parts support, public transit plus active mobility, a community shuttle, or a declared combination. The benchmark must allow "not deployable yet" and "none of the options qualify" as valid outcomes.

The IEA's Global EV Outlook 2026 notes regionally divergent adoption and the significance of two- and three-wheelers in some markets. The data support keeping the candidate space broader than four-wheel passenger vehicles; they do not imply that a cargo cycle, EV, hybrid or shared mode is universally preferred. See the [2026 executive summary](https://www.iea.org/reports/global-ev-outlook-2026/executive-summary) and [other EV modes](https://www.iea.org/reports/global-ev-outlook-2026/trends-in-other-ev-modes).

## Algorithm

### 1. Scope one comparison

All candidates must share the exact scenario ID, service unit, intended user segment, gate definition set, metric definition set, units and metric directions. Do not mix a per-household monthly mobility service with a per-vehicle kilometre result or mix currencies/denominators without a separately defined conversion and evidence record.

### 2. Apply non-tradeable hard gates first

Every declared gate is mandatory and has one disposition:

- `fail` -> candidate is `blocked`;
- `unknown` with no failed gate -> `needs_evidence`;
- every gate `pass` -> candidate may be considered for ranking, if its metrics are complete.

No metric value compensates for a failed legal/operating-scope, essential-access, energy/service, repair/recovery, or privacy/owner-control gate. Gate dispositions in the fixture are synthetic test labels only; they are not evidence of actual approval or physical safety.

### 3. Keep missingness distinct from value

Every candidate must supply the same metric IDs. Metric units must exactly match the comparison definition. Numeric fixture values are explicitly `synthetic_value`; unavailable values have `status: unknown` and `value: null`. Missing metric records, duplicate IDs, unknown fields, wrong units, and non-finite values reject the input. Unknown metric values do not become zero and candidates with incomplete metrics are not placed on the Pareto frontier.

### 4. Report Pareto sets, not a hidden composite score

Among all-hard-gates-pass candidates with complete comparable metrics, candidate A dominates B only if A is no worse on every declared metric and strictly better on at least one, accounting for each metric's direction. The frontier is exact only for the finite candidate set and declared synthetic metric definitions. Ties remain non-dominating rather than being broken into a fake global quality rank.

The output reports:
- every candidate's hard-gate disposition and explicit findings;
- missing metric IDs and raw synthetic metric values;
- which candidates dominate an eligible non-frontier option;
- the Pareto-frontier candidate IDs;
- an explicit statement that no scalar utility score or universal winner was produced.

There is no implicit objective weighting. Decision owners must explain any later policy choice among Pareto alternatives rather than bury values in a weighted score.

## v0.1 fixture

The one synthetic scenario uses an abstract monthly essential-trip service unit and includes eight option bundles:
1. compact EV + shared/public charging;
2. compact hybrid + local service;
3. repairable used vehicle + parts pool;
4. cargo cycle + transit;
5. community shuttle;
6. cheap but uncertified import (hard-gate negative control);
7. option with unconfirmed energy and repair support;
8. low-initial-burden option with unknown lifecycle/monthly burden.

All input numbers are constructed solely to exercise the evaluator. They are not measured prices, trip coverage, repair downtime, access times, environmental indicators, or vehicle performance. The fixture uses a normalized budget ratio and normalized lifecycle index rather than pretending to have currency or audited life-cycle data.

## Independent tests and targets

The Rust crate includes tests for:
- hard-gate failures remaining excluded even with artificially ideal metrics;
- unknown gates requiring evidence;
- unknown metric values remaining null and unranked;
- unit mismatches and undeclared/duplicate fields failing closed;
- candidate/gate/metric ordering yielding byte-identical serialized reports;
- Pareto agreement with an independently implemented oracle for every four-point subset of the 3D binary cube (70 candidate subsets).

Proposed future gates:
- 100% agreement with the independent exhaustive oracle on the bounded fixture family;
- 0 hard-gate failures admitted to ranking;
- 0 unknown-to-zero imputations;
- 100/100 input-order permutations produce identical canonical reports;
- 0 scalar "best vehicle" scores emitted by this evaluator.

These are authored test expectations and targets, not claims that Rust tests have run. Use `cargo test --manifest-path tools/mobility-option-evaluator/Cargo.toml` and `cargo run --manifest-path tools/mobility-option-evaluator/Cargo.toml -- fixtures/mobility_options/mobility_option_scenarios_v0_1.json` in a Rust-enabled environment.

## Evidence and claim ceiling

The European Commission's [vehicle repair and maintenance information guidance](https://single-market-economy.ec.europa.eu/sectors/automotive-industry/technical-harmonisation/access-vehicle-repair-and-maintenance-information_en) documents EU rules supporting independent repairer access; those rules are jurisdiction-specific and not a global legal claim. [ISO 59020:2024](https://www.iso.org/standard/80650.html) provides a framework for circularity measurement and transparent system boundaries; it is not a vehicle qualification.

Before real use, replace synthetic metrics with a separate, rights-cleared observation layer binding each value to source, timestamp, scope, units, uncertainty, conversion, owner, and validity period. Keep observed measurements separate from supplier quotes, simulations, and modeled estimates. Exact JSON output or source hashes support reproducibility, not physical truth.

This evaluator is neither a purchase recommender nor a dispatch controller. It must never authorize real driving, autonomous action, procurement, or market release. Applicable local engineering, safety, legal, user and regulator review remains necessary.
