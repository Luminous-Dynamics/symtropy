# Published shipyard scheduling baselines to reproduce

This file pins the first directly relevant published numerical anchor for the space-time placement/recovery work. It is a comparison specification, not a claim that our current model reproduces the published problem.

## Primary paper

Ningfei Wang, Jingbo Yin, Rafi Ullah Khan, “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding,” *IET Collaborative Intelligent Manufacturing*, first published 11 July 2026, DOI: https://doi.org/10.1049/cim2.70066.

The paper formulates irregular-block planar storage yard (PSY) layout and scheduling as a mixed-integer model, maximizing yard utilisation and minimizing vehicle scheduling time. Its geometric method uses supplementary-angle and supplementary-edge layout rules; its optimizer combines a two-stage artificial bee colony (ABC) method, adaptive large-neighbourhood search (ALNS) destroy/repair, and an explicit disruption-recovery step. The evaluation uses real shipyard cases and compares a proposed algorithm (PA) with traditional ABC (IA), genetic algorithm (GA), and differential evolution (DE).

## Published numerical anchor

Table 6, Case9_50_4, adds four disrupted blocks within the study's sequence of short-term plans; its disruption-type columns show two workstation disruptions and two vehicle disruptions. The paper reports 15 repetitions per case under identical parameter settings.

| Method | Yard utilisation reported (Aur) | Algorithm time (Act, s) | Vehicle scheduling/travel time (Vt, min) |
|---|---:|---:|---:|
| Proposed hybrid PA | 0.775 (~77.5%) | 308.0 | 2,339 |
| Traditional ABC (IA) | 0.765 (~76.5%) | 398.7 | 2,339 |
| Genetic algorithm (GA) | 0.760 (~76.0%) | 335.6 | 2,386 |
| Differential evolution (DE) | 0.768 (~76.8%) | 352.1 | 2,360 |

Source: the paper's Table 6 and accompanying discussion at https://ietresearch.onlinelibrary.wiley.com/doi/10.1049/cim2.70066. The Aur values are copied as reported; do not infer extra precision. Act is the study's algorithmic computation time, not a real-world operational delay. Reproduce the instance definition, objective weights, time accounting, travel-time inputs and stopping conditions before a numeric comparison.

The same paper states that its two objective functions use the weighted-goal criterion with weights 0.5 each, based on yard-manager assessment. This is a particular preference choice, not a universal industry utility function. Any replication should preserve the published weighting for a headline reproduction and add sensitivity analysis for alternative space/time trade-offs.

## Model boundaries to turn into adversarial cases

The paper's stated assumptions include:
- blocks being moved out are treated as fully relocated when new blocks arrive; interactions between outgoing and incoming blocks are not considered;
- ordering among blocks entering in the same short-term plan is not considered;
- loading/unloading times are neglected;
- workstation-to-yard travel times are treated as constant even though vehicle departure times vary;
- the yard has an adequate number and variety of vehicles for block transport.

Those are defensible modelling simplifications for a bounded study, but they define high-value follow-on experiments for a more operational digital twin:

1. **Handoff semantics:** a block's old footprint remains occupied until lift-off/relocation confirmation; the incoming footprint becomes available only after a measured handoff event. Test near-simultaneous arrivals against zero-time transfer assumptions.
2. **Finite, qualified fleet:** vehicle availability, capacity, compatibility, operator qualification, refuelling, and concurrent claims are explicit constraints. Run scarce-fleet and equipment-outage controls.
3. **Real transfer resources:** add loading/unloading, turning/swept path, access corridor, travel-time uncertainty and destination occupancy only when the input model can represent them; unsupported dimensions must fail closed.
4. **Irregular shapes:** compare rectangle approximation against validated polygons or conservative envelopes and report unusable-envelope area. The current V0 verifier supports rectangles only; it cannot reproduce the paper's geometric strategy.
5. **Recovery stability:** freeze what has already happened; count unaffected-plan churn, re-handles, recovery horizon, objective regret and makespan/tardiness separately. Keep a Pareto frontier rather than hiding trade-offs in a single unexamined score.

## Fair reproduction protocol

Before running a head-to-head:
- obtain or reconstruct the same case geometry, block dimensions/polygons, process/arrival sequence, resource assumptions, disruption schedule, and distance/time matrices;
- pin the instance source, units, transformations, random seeds, code revision and all objective weights;
- run 15 repetitions for stochastic methods if replicating the paper's evaluation, and report mean, median, dispersion and best/worst cases rather than a single best run;
- give every method the same time/evaluation budget and hardware, and report algorithm time separately from verifier/IO time;
- report yard utilisation, vehicle scheduling/travel time, makespan/tardiness, feasibility, recovery horizon, unaffected reservation churn and compute/memory separately;
- validate every candidate with the same independent space-time verifier;
- label missing data as non-comparable. Do not invent a numeric reproduction from the paper's summary table.

The paper is a strong next comparator, but this repository's rectangle-only checker and first-fit policies are not a reproduction of the paper's irregular-polygon MILP/ABC/ALNS model. The first-win milestone is a faithful, pinned reproduction; the later-win milestone is statistically credible improvement on the same frozen instances plus additional operationally realistic controls.
