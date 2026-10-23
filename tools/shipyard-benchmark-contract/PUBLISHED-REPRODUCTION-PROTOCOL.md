# Published shipyard benchmark reproduction protocol

**Status:** protocol and source audit only; no published industrial instance has yet been reproduced.

## Reference and available evidence

Primary reference: Ningfei Wang, Jingbo Yin, and Rafi Ullah Khan, “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding,” *IET Collaborative Intelligent Manufacturing* (first published 11 July 2026), [DOI 10.1049/cim2.70066](https://doi.org/10.1049/cim2.70066).

The paper formulates irregular-block planar storage-yard placement and scheduling under workstation and vehicle disruption. Its hybrid method combines a two-stage artificial bee colony algorithm, adaptive large-neighbourhood search, and disruption-recovery rules. The paper compares its proposed method (PA) with ABC, genetic algorithm, and differential evolution.

**Data availability boundary:** the article states that the experimental data and simulation results supporting its findings are available from the corresponding author upon request. The published page is not a downloadable, complete machine-readable benchmark bundle. Do not claim a faithful reproduction by recreating approximate cases from its headline metrics.

## Published numerical anchor to reproduce

The article reports 15 repetitions per disruption case and gives the following values for Case9_50_4:

| Method | Reported yard utilisation (Aur) | Algorithm time (Act, seconds) | Vehicle scheduling/travel time (Vt, minutes) |
|---|---:|---:|---:|
| Proposed hybrid PA | 0.775 | 308.0 | 2,339 |
| Traditional ABC (IA) | 0.765 | 398.7 | 2,339 |
| Genetic algorithm (GA) | 0.760 | 335.6 | 2,386 |
| Differential evolution (DE) | 0.768 | 352.1 | 2,360 |

These values are targets for reproduction, not values for direct comparison with the current rectangular baseline. The paper uses irregular-block representations and an objective weighting of 0.5 for yard utilisation and 0.5 for vehicle scheduling time. The exact normalization and inputs must be preserved before treating the weighted objective as comparable.

## Published context available in the paper

The paper describes Case9 as a 50-block setting, five short-term subcycles, and a 70 m × 50 m yard, with six workstations. Across its generated case families it also varies yard length, block count and subcycle count. Its reported modelling context includes vehicle wait times of 10–30 minutes, refuelling time of 20 minutes, workstation-to-yard travel times of 40–60 minutes, 20 minutes between adjacent workstations, and block transport durations of 80–100 minutes. Disruptions are generated across short-term plans.

The appendix includes a finite collection of example block geometries and vehicle types. That is useful for creating a separately attributed geometry smoke-test corpus, but it is not by itself the complete 50-block Case9_50_4 benchmark. Check the article's reuse terms before copying its coordinate table into this repository. Prefer a source link and transformation script where licence status or reuse scope is not confirmed.


## Published shape-approximation baseline: quantify geometry-induced waste

Table 4 of the paper separately compares source/initial block profiles (R1), simplified profiles (R2), and regularized shapes (R3). It reports added footprint area relative to initial shapes:

| Case size | Initial-shape area (m²) | Simplified R2 area (m²) | R2 inflation | Regularized R3 area (m²) | R3 inflation |
|---|---:|---:|---:|---:|---:|
| 40 blocks | 1,960.83 | 2,086.06 | 6.39% | 2,252.89 | 14.89% |
| 50 blocks | 2,438.20 | 2,593.43 | 6.37% | 2,871.25 | 17.76% |
| 60 blocks | 2,916.81 | 3,097.90 | 6.21% | 3,434.75 | 17.75% |

Source: Table 4 in https://doi.org/10.1049/cim2.70066. These are the study's geometry-representation area deltas, not the yard-utilization percentages in Table 6. They do not mean the study's total utilization is reduced by exactly the same percentage: yard aisles, shape placement, and the objective's other terms remain separate.

This creates a second benchmark axis in addition to optimizer quality:

1. **Representation loss:** `(approximate footprint area - source profile area) / source profile area`. For exact fixed-point input polygons, target 0% added area from geometry simplification, while recording quantization and clearance inflation separately.
2. **Packing quality:** actual source-profile area placed inside the qualified usable yard, divided by the published/declared usable yard area. Report this separately from representation loss.
3. **Conservatism:** number of valid layouts rejected because bounding envelopes overlap while the source polygons are disjoint. The synthetic V0 L-shape counterexample is a semantic demonstration of this failure mode, not a proxy for the paper's entire industrial case.

The prototype's three independently authored synthetic orthogonal shapes have bounding-box area ratios 1.5x–1.8x (50%–80% inflation). This adversarial set intentionally emphasizes concavity; do not compare its ratios directly to the paper's case-wide Table 4 values. It is useful for catching methods that conflate an AABB broadphase with authoritative footprint geometry. The fair next experiment is to acquire or obtain authorization for the original source profiles, run the same profile set through R1/R2/R3-style normalization and exact fixed-point geometry, and report profile-level area deltas plus final yard-placement metrics separately.

**Target:** preserve every source profile without geometric-area inflation beyond the declared fixed-point quantization tolerance; do not claim improved industrial utilization until the same profiles, yard aisles, objective definitions and disruption schedule have been evaluated by the matched placement/recovery benchmark.

## Known modelling assumptions to challenge after faithful reproduction

The article explicitly describes simplifying assumptions that are important for operational robustness:
- an outgoing block is treated as fully relocated when an incoming block arrives;
- ordering between blocks entering within one short-term plan is not represented;
- loading and unloading time is neglected;
- workstation-to-yard travel time is treated as constant despite vehicle departure time variation;
- vehicle fleet quantity and variety are assumed sufficient for the block types in the problem;
- one vehicle transports one block at a time.

Do not silently “fix” these assumptions in the headline reproduction. First reproduce the reference model; then add separately labelled ablations to quantify the impact of delayed clearance, finite/qualified fleets, loading and unloading, time-dependent travel, swept paths, and multi-block transport.

## Reproduction protocol

1. **Request and preserve source data.** Ask the corresponding author for the Case9_50_4 source instance, coordinate/profile representation for all 50 blocks, process/subcycle order, vehicle fleet/capabilities, workstation coordinates and travel-time matrix, disruption event identities/times, algorithm configurations, objective normalizations, termination criteria, and data/licence conditions. Record the request/permission status; never store private operational inputs in the public repository without authorization.
2. **Create a content-addressed bundle.** Once authorized, record a canonical manifest, per-file SHA-256 digests, units, coordinate transforms, time conventions, geometry semantics, random seeds, method source revisions, and the exact raw-to-normalized transformation. Preserve raw files outside the public repo where licensing requires it; publish checksums and a schema-valid synthetic test when possible.
3. **Rebuild the paper methods.** Match each method's published definition and parameters. Do not substitute the present first-fit portfolio, generic ALNS, or another method and call it a reproduction.
4. **Match the experiment.** Run 15 matched repetitions per stochastic method on identical instances/disruption traces, with the source paper's hardware or a justified normalized compute protocol. Pin the seed list and budget before inspecting outcomes.
5. **Validate every plan.** Use one independent constraint checker shared by all solvers, with oracle-backed tiny controls. Separate solver time from IO, exact verification, and report serialization.
6. **Report distributions and vectors.** Report mean/median/dispersion and best/worst across repetitions; yard utilisation, travel, makespan/tardiness, recovery horizon, unaffected-plan churn, re-handles, feasibility, runtime and memory remain separate. Do not infer statistical significance from a single run or the published rounded aggregate.
7. **Add operational ablations.** Freeze a second corpus with observed-clearance delays, finite fleet outages, loading/unloading durations and scenario sequences. Measure how often the source-paper assumptions alter feasibility, objective quality and recovery stability.
8. **Make claims fail-closed.** If any required raw input is unavailable, label the comparison “partial reproduction” or “not comparable.” Never infer missing block geometry, schedules, travel matrices, or objective normalizations from the reported output values.

## Immediate targets

- **R0 — provenance:** a manifest identifying exactly which source inputs are available, requested, authorized, missing, or synthesized.
- **R1 — semantic:** zero false-feasible and false-infeasible outcomes on the declared exact tiny-domain oracle, including negative controls.
- **R2 — reproduction:** reproduce the paper's Case9_50_4 metrics within a predeclared numerical tolerance and report all 15 repetitions; do not set a tolerance until the paper's metric definitions and rounding are confirmed.
- **R3 — fair method comparison:** run matched ABC/GA/DE/hybrid-PA baselines or faithfully reconstructed equivalents with identical instance, seed schedule, budgets, and validation.
- **R4 — operational advance:** add delayed handoff and finite-resource ablations as separate, independently verified experiments.
- **R5 — industrial evidence:** obtain a permissioned source-pinned execution trace and validate real yard KPIs over a declared evaluation horizon.

## Source references

- Primary paper: https://doi.org/10.1049/cim2.70066
- Article page and data availability statement: https://ietresearch.onlinelibrary.wiley.com/doi/10.1049/cim2.70066
- Multi-crane later comparator (not comparable until crane-interference and tandem constraints are modelled): https://doi.org/10.1016/j.asoc.2026.115384
- Shipyard digitalisation review: https://doi.org/10.3390/jmse14040396
