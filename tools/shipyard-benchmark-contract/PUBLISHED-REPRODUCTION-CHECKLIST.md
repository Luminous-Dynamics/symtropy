# Published shipyard benchmark reproduction checklist

Status: protocol prepared; the published irregular-block experiment is NOT yet reproduced.

## First target

Wang, Yin, and Khan, “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding,” IET Collaborative Intelligent Manufacturing, first published 11 July 2026. DOI: https://doi.org/10.1049/cim2.70066.

The existing PUBLISHED-BASELINES.md records the paper's published Case9_50_4 anchor: the proposed hybrid method reports yard utilisation 0.775, algorithm time 308.0 seconds, and vehicle scheduling/travel time 2,339 minutes. The paper reports 15 repetitions per case and compares its method to ABC, GA, and DE. These numbers are reference values, not local reproduction results.

## Required artifacts before a numeric comparison

- Exact instance definitions: irregular block polygon vertices (or published shape representation), units, block IDs/dimensions, yard bounds, workstation/dock anchors, entry/exit sequence, and permitted placements.
- Exact process and disruption data: short-term plan sequence, the four disrupted blocks, two workstation disruptions, two vehicle disruptions, and the observation/recovery point used in Case9_50_4.
- Exact travel inputs: distance/time matrix, vehicle speed/availability assumptions, loading/unloading times and any simplifications retained by the paper.
- Exact objective: published utilization definition, vehicle-time definition, normalization, weight values, and stopping conditions. Preserve the paper's 0.5/0.5 weighted-goal criterion for headline replication, then run clearly labelled weight-sensitivity analysis separately.
- Exact comparison protocol: 15 seeded repetitions for stochastic methods when reconstructing the paper's protocol; identical instance input and time/evaluation budgets; hardware/software revision; best, median, mean, dispersion, feasibility, and per-run wall time.
- Reproduction bundle: raw-input digest, normalized canonical instance digest, unit conversion record, parser version, solver source revision, configuration digest, run report, and independent feasibility verification.

## High-value assumption stress tests

The paper's baseline assumptions include immediate relocation of outgoing blocks, no ordering among simultaneously arriving blocks in a short-term plan, neglected loading/unloading times, constant workstation-to-yard travel times, and adequate vehicle variety/quantity. Treat these as separate ablations, never silently alter the headline reproduction.

1. Handoff reality: old footprints remain occupied until a measured lift-off/clearance event. Vary late clearance and near-simultaneous arrivals.
2. Scarce fleet: finite vehicle inventory, explicit qualification/capability, shift calendars, and planned vehicle outages.
3. Travel realism: depart-time-dependent travel and load/unload/setup duration, only when source inputs support it.
4. Geometry fidelity: compare rectangle/conservative-envelope approximations to the source irregular shapes and report unusable area.
5. Recovery stability: preserve completed/active work; measure unaffected-plan churn, re-handles, recovery horizon, makespan/tardiness, and regret independently.

## Missing-data rule

If the article's full instance tables, irregular-geometry representation, or disruption schedule cannot be obtained from the publication or an authorized source, mark the run non-comparable. Do not reverse-engineer unstated coordinates, fabricate travel times, or infer the paper's random seeds from aggregate results. A synthetic analogue may be useful engineering work, but must be labelled synthetic and cannot be called a reproduction.

## Separate generic-scheduling track

Use JSPLib (https://scheduleopt.github.io/benchmarks/jsplib/) and FJSPLib (https://scheduleopt.github.io/benchmarks/fjsplib/) only for the discrete scheduling subproblem they actually represent. Their published instance libraries and best-known bounds are useful for checking precedence/resource scheduling, but they do not validate irregular yard geometry, transfer corridors, crane interference, or physical safety.

## Success criteria

A first successful reproduction is not “our output is close”: it means the same instance, objective, assumptions, disruptions, repetition protocol, and scoring definitions have been recovered and all candidate plans have passed a common independent verifier. Only after that may we test whether a new method improves matched-instance quality or runtime. Industrial superiority requires separately authorized operational traces and longitudinal KPI validation.
