# Marine inter-sample envelope: state of practice and benchmark position

**Date:** 19 November 2026  
**Track:** Symtropy #6293; stacked on trajectory audit #6261 and liveaboard service-vector #6251

## Why the next boundary matters

The finite-horizon trajectory auditor checks every declared state sample and adjacent conservation ledger. A finite sample grid alone does not guarantee that the continuous state between samples stays inside a service floor. This is a sampled-data assurance gap, not a forecast-accuracy problem.

The present increment adds a deterministic, declared-rate envelope over each adjacent sample interval. It takes uncertainty intervals at both endpoints and a mission-specific maximum absolute rate bound. A lower bound is used for reserve/service minima; an upper bound is used for maximum exposure/workload ceilings. Discrete mode changes must occur at declared time-grid boundaries; they cannot be hidden by smoothing a hybrid event into a continuous derivative limit.

## Relevant current work

### Segment-safe inter-sample constraints

Pagnini and Malis, *Segment-safe control barrier functions for model predictive control*, European Journal of Control, online 23 July 2026, DOI https://doi.org/10.1016/j.ejcon.2026.101582. The authors explicitly target safety along the segment between sampled states and compare against zero-order-hold and standard discrete CBF approaches. Their validation is on nonlinear quadrotor dynamics, not a vessel liveaboard-resource model.

**Implication:** endpoint-only safety is insufficient; a segment-level constraint is an active research direction. Do not claim the simple scalar envelope below is equivalent to their CBF/controller guarantees.

### Finite-sample reachability and probabilistic barriers

Prajapat et al., *Finite-sample-based reachability for safe control with Gaussian Process dynamics*, Automatica 193 (November 2026), article 113204, DOI https://doi.org/10.1016/j.automatica.2026.113204, studies finite-sample reachable-set construction when the dynamics are modelled using a Gaussian Process.

*Robust data-driven MPC with analytic probabilistic safety barriers* (September 2026), DOI https://doi.org/10.1016/j.rico.2026.100789, proposes analytic Lipschitz-based safety margins with separate epistemic and aleatoric uncertainty treatment and recursive-feasibility results under its stated assumptions.

**Implication:** the research path beyond this scalar deterministic tool is a validated hybrid reachable tube with calibrated model uncertainty and explicit control/disturbance sets. The current implementation only uses a declared derivative/rate ceiling; it does not validate that ceiling or produce a probabilistic guarantee.

### Field-validated shipboard digital twins

Kondratyev et al., *Field-Validated Probabilistic Digital Twin of a Shipboard Microgrid for Reliability-Oriented Operation: Applications to Hybrid and Unmanned Vessels*, Energies 2026, 19(17), 4074, DOI https://doi.org/10.3390/en19174074. The study reports 162,360 time samples at a 4-second cadence, 12 operating/fault classes and 180 hours of field observation. Reported comparative outcomes include 37.3% lower voltage RMSE, 32.7% lower SOC RMSE and 30.1% lower power RMSE against its deterministic twin, plus a 13.3% reduction in energy not served. The paper explicitly bounds transfer: the data are from one operator and predominantly conventional tanker vessels, so hybrid/unmanned platform use needs prospective onboard validation.

**Implication:** do not claim a generic probabilistic shipboard digital twin as novel. A stronger differentiation is an independent verifier that consumes telemetry/model outputs, preserves the provenance of rate bounds and rejects unobserved inter-sample service breaches. A real benchmark should use high-rate telemetry where available (e.g. 4-second class data), not extrapolate from the synthetic six-hour examples in this fixture.

### Integrated multi-energy ship systems already exist

The September 2026 *Rolling horizon energy management of a green multi-energy ship microgrid with voyage routing and waste heat recovery*, DOI https://doi.org/10.1080/20464177.2026.2728778, coordinates electricity, thermal energy, hydrogen and freshwater while considering adverse-weather sailing resistance and generator waste-heat recovery.

**Implication:** simply coupling freshwater with ship energy dispatch is not a novelty claim. The target here is independent, evidence-bound safety qualification of the *continuous* service trajectory across intervals and registered hybrid events.

## Exact envelope used

For a minimum constraint, let a and b be the conservative lower bounds at the two endpoints, let L be a declared maximum absolute derivative in metric-units/hour, and let dt be the timestamp-derived interval duration in hours. The endpoint cones imply:

`x(t) >= max(a - L t, b - L(dt-t))`

The auditor computes the exact minimum over the interval, using the cone intersection if it lies inside the interval and the corresponding endpoint case otherwise.

For a maximum constraint, using upper endpoint bounds a and b:

`x(t) <= min(a + L t, b + L(dt-t))`

The auditor computes the exact maximum similarly. These are conservative scalar envelopes under the declared Lipschitz/rate assumption. The two endpoint uncertainty sets must also be compatible with the rate ceiling.

Each rate ceiling is part of the mission-specific canonical service contract and carries a method, evidence digest and evidence class. Synthetic rate bounds cannot be promoted to measured-campaign evidence. Known outage/repair/weather/operator/shore-service events must occur exactly at a declared segment boundary; the interval is split at every registered event.

## What this adds—and what it does not

Adds:
- unit-bearing scalar inter-sample upper/lower envelopes for eleven resource/service axes;
- two-endpoint uncertainty propagation;
- mission-bound rate ceilings with evidence identity;
- exact event-boundary and scenario roster requirements;
- independent known-answer controls where sampled endpoints pass but the segment envelope fails;
- worst envelope, floor margin, interval and evidence references.

Does not add:
- physical validation of the derivative ceilings;
- nonlinear multi-state reachability;
- controller synthesis or a safety filter;
- recursive feasibility or a viability-kernel proof;
- probability calibration, HIL/PHIL results or operational authorization.

## Benchmark protocol

1. **Hand-oracle correctness:** 100% agreement with closed-form, piecewise two-cone answers over a broad deterministic grid of endpoint pairs, rates and interval durations.
2. **Dense trajectory falsification:** compare the envelope to high-resolution continuous synthetic paths satisfying the same rate ceiling. Target zero false accepts; report false rejects separately.
3. **Baseline comparison:** endpoint-only, midpoint-only, min/max-of-endpoints, nearest-endpoint Lipschitz and exact two-cone envelope. Report false-accept rate and conservatism by axis.
4. **Scenario coverage:** 100% scenario/time-grid/registered-event coverage; zero hidden event boundaries.
5. **Evidence integrity:** 100% digest agreement for rate contracts, mission snapshot, initial state, time grid, events and complete scenario payloads.
6. **Execution:** report p50/p95/p99 latency and memory versus scenario count × interval count × metric count.
7. **Physical progression:** identify rate ceilings from high-rate target-vessel data; calibrate sensor/model uncertainty; independently validate the operating domain; then HIL/PHIL and staged sheltered-water trials.

Research target: **0 accepted conservative envelope breaches and 0 accepted omitted event boundaries**. Performance claims require independent data, adequate sample sizes and paired voyage/weather-event analysis. The synthetic fixtures only establish deterministic software behavior.
