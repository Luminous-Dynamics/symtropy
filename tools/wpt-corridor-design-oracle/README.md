# WPT exact corridor siting and N-1 service oracle v0

Tracks [WPT corridor co-design #3861](https://github.com/Luminous-Dynamics/symtropy/issues/3861). This is a planning-level companion to the temporal service oracle and the multi-objective dispatch oracle.

## Paradigm shift: site for guaranteed route energy, not coil density

The decision is a set of installed roadway segments. A candidate segment connects one feeder to one or more routes, has a maximum deliverable-energy capacity, and has a capital-cost unit. A route has an individual minimum useful-energy requirement.

For each design subset, the evaluator builds a time-agnostic capacity network:

`source → feeder → installed segment → route → sink`

The same feeder supply and segment output budgets constrain all served routes jointly. A max-flow equal to the sum of route service floors is a constructive witness that every route floor can be met simultaneously.

It then checks:
1. Normal operation.
2. Every single installed-segment outage, one at a time.
3. Every declared feeder outage, one at a time, with the remaining independent supply and installed modules reallocated to the routes.

It enumerates every non-empty segment subset and reports all minimum-capex designs that satisfy the complete normal + one-at-a-time N-1 segment- and feeder-outage set.

## Known-truth fixture

- 2 feeders, each with 9 kWh available.
- 2 routes: route-east requires 5 kWh and route-west requires 4 kWh.
- 6 candidate segments with different feeder assignments, route coverage, capacity and capex.
- 63 non-empty designs exhaustively checked.
- 25 designs satisfy normal operation, all four selected-segment outages and both feeder outages.
- Minimum capex is 16 synthetic cost units; the unique minimum design is `a-route-east`, `a-route-west`, `b-route-east`, and `b-route-west`.
- Seven service checks accompany the selected design: normal, four selected-segment outages and two feeder outages. Every route receives its full 5 / 4 kWh minimum in every check.

The cheaper shared-segment alternative fails the feeder-outage or module-outage checks because it can leave both routes dependent on one common feed or module. The exact search chooses one dedicated module per route on each independent feeder.

## Exactness and limits

- Candidate sets are bounded at 12 segments, 4 feeders and 8 routes (`2^12 - 1` non-empty designs maximum).
- Energy and capex are integer kWh / integer cost units.
- Flow enforces shared feeder and segment caps across multiple routes.
- Each outage scenario removes one installed segment or one feeder and solves a fresh max-flow recourse allocation. Segment and feeder outages are assessed one at a time, not as simultaneous failures.
- No heuristic or truncation is used; every subset is evaluated.
- Input row order does not change selected IDs, minimum cost or robust feasible count.

## What it does not model

This v0 only proves service floors in a static aggregate capacity network under **single segment or single feeder outages, one component at a time**. It does not protect against:
- source/substation or upstream common-mode failure, multiple simultaneous component failures, flood/fire/road closure or correlated hazards;
- time-dependent vehicle trajectories, dwell, handover, arrival/departure windows or traffic;
- coil pitch, magnetic coupling, nonlinear multiport interaction, cross-regulation, switching, thermal state or harmonic constraints;
- nonlinear AC power-flow, voltage stability, protection coordination, EMC or safety;
- pavement fatigue, moisture, freeze-thaw, repair access, construction staging or measured asset survival;
- local CAPEX, maintenance, land, permitting, tariff or financing.

Those require separate validated models and evidence. A linear feeder/segment flow graph is not an electromagnetic or AC-network model.

## Validation

`cargo test --locked -p wpt-corridor-design-oracle`

Eight Rust tests cover the exact known-truth design, normal-only versus N-1 infeasibility, outage enumeration, invalid feeder/route references, duplicate/unknown/non-synthetic controls, input-order invariance and integer cost/capacity contracts.

## Claim ceiling

Exact minimum-capex selection for the finite synthetic capacity graph with one-at-a-time N-1 segment/feeder contingencies only. It does not establish real-world corridor payback, road lifetime, grid compliance, physical interoperability, safety, field service or deployment authorization.
