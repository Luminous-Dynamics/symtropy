# Published shipyard baseline: reproduction protocol v0

Status: reproduction target only. No published instance has been reproduced by this repository yet.

## Primary reference

Ningfei Wang, Jingbo Yin, and Rafi Ullah Khan, “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding,” *IET Collaborative Intelligent Manufacturing* (first published 11 July 2026), DOI: https://doi.org/10.1049/cim2.70066.

The paper models the layout and scheduling of irregularly shaped planar-storage-yard blocks under workstation and vehicle disruption. It combines a two-stage artificial bee colony (ABC) method, adaptive large neighbourhood search (ALNS), and a disruption-recovery module. Its primary reported objectives are planar storage yard (PSY) utilisation and vehicle scheduling/travel time. The paper states that experimental data and simulation results are available from the corresponding author on request.

## Table 6 reference points

The published table reports PA (proposed hybrid ABC+ALNS+recovery), IA (traditional ABC), GA, and DE on Case9_50_x. The values below are copied as a target for reconstruction, not measured by this repository.

| Instance | Disturbance type D1 / D2 | Method | PSY utilisation Aur | Algorithm time Act (s) | Vehicle travel Vt (min) |
|---|---:|---|---:|---:|---:|
| Case9_50_0 | 0 / 0 | PA | 0.782 | 119.4 | 2,229 |
| Case9_50_0 | 0 / 0 | IA | 0.782 | 232.5 | 2,229 |
| Case9_50_0 | 0 / 0 | GA | 0.782 | 185.3 | 2,229 |
| Case9_50_0 | 0 / 0 | DE | 0.782 | 201.6 | 2,229 |
| Case9_50_1 | 1 / 0 | PA | 0.782 | 223.6 | 2,276 |
| Case9_50_1 | 1 / 0 | IA | 0.782 | 316.7 | 2,276 |
| Case9_50_1 | 1 / 0 | GA | 0.782 | 268.4 | 2,276 |
| Case9_50_1 | 1 / 0 | DE | 0.782 | 285.2 | 2,276 |
| Case9_50_2 | 1 / 1 | PA | 0.777 | 248.4 | 2,298 |
| Case9_50_2 | 1 / 1 | IA | 0.777 | 388.01 | 2,298 |
| Case9_50_2 | 1 / 1 | GA | 0.777 | 295.7 | 2,298 |
| Case9_50_2 | 1 / 1 | DE | 0.777 | 312.6 | 2,298 |
| Case9_50_3 | 1 / 2 | PA | 0.768 | 271.6 | 2,315 |
| Case9_50_3 | 1 / 2 | IA | 0.761 | 306.2 | 2,327 |
| Case9_50_3 | 1 / 2 | GA | 0.758 | 284.9 | 2,368 |
| Case9_50_3 | 1 / 2 | DE | 0.764 | 297.3 | 2,342 |
| Case9_50_4 | 2 / 2 | PA | 0.775 | 308.0 | 2,339 |
| Case9_50_4 | 2 / 2 | IA | 0.765 | 398.7 | 2,339 |
| Case9_50_4 | 2 / 2 | GA | 0.760 | 335.6 | 2,386 |
| Case9_50_4 | 2 / 2 | DE | 0.768 | 352.1 | 2,360 |
| Case9_50_5 | 3 / 2 | PA | 0.772 | 350.9 | 2,375 |
| Case9_50_5 | 3 / 2 | IA | 0.770 | 401.7 | 2,361 |
| Case9_50_5 | 3 / 2 | GA | 0.764 | 372.8 | 2,412 |
| Case9_50_5 | 3 / 2 | DE | 0.769 | 389.5 | 2,390 |

The paper describes 15 repetitions for each sensitivity-analysis setting. Before reproducing aggregate statistics, obtain the per-run raw outputs or establish the exact repetition protocol; a mean reconstructed from printed table cells is not a reproduction.

## Reconstruction gate

Do not compare scores until all of these are pinned:

1. **Instance identity:** each block's ordered polygon vertices, orientation rules, usable-yard boundary, aisle subtraction, and clearance spacing. Publish exact artifact hashes.
2. **Initial state:** workstation completion times, initial block sequence, reserved/in-flight placements, already distributed blocks, and which reservations are immutable after a disruption.
3. **Resource model:** vehicle identities/types, load capacities, workstation eligibility, travel-time matrix, availability calendars, vehicle outage timing, and refuelling/replacement assumptions.
4. **Timing semantics:** units and event ordering; loading/unloading durations; concurrent-arrival order; whether departure/arrival times occupy the yard; handling of outgoing blocks that have not cleared.
5. **Objectives:** exact Aur numerator and denominator; exact Vt aggregation boundary; the paper's weighted goal criterion with weights 0.5 / 0.5; tie-breaking and feasibility penalties. Keep raw Aur, Vt, runtime, and constraint status separate even when reproducing the original scalarized optimizer.
6. **Algorithm parameters:** PA's ABC nectar sources (SN=30) and maximum iterations (MaxIter=200), all ALNS destroy/repair operators and acceptance parameters, recovery rules, GA/DE settings, stopping criteria, random seed algorithm and repetitions.
7. **Runtime environment:** CPU model, core count, RAM, compiler/runtime versions, thread count, and exact budget/stopping rule. Published wall-clock seconds are not a fair target when hardware or stopping rules differ.
8. **Output evidence:** initial and final layouts, operation order, vehicle assignments/travel legs, all hard-constraint checks, per-run metrics, solver exit status, and raw execution receipt.

## Assumption-relaxation experiments

The paper explicitly assumes that outgoing blocks have already cleared when incoming blocks arrive, neglects loading/unloading durations, does not constrain the order of blocks arriving concurrently, and assumes sufficient vehicles of suitable types. These assumptions are legitimate for its declared model but create specific follow-on experiments:

- **Clearance-aware occupancy:** retain an outgoing block's footprint until a witnessed, source-backed clearance event. Test whether schedules that pass the paper's simplified model fail when departure-to-clearance delay is included.
- **Loading / unloading time:** add explicitly modelled lift/handling duration and resource occupation, preserving the original result as the zero-duration baseline.
- **Concurrent arrivals:** enumerate the event order or jointly schedule arrivals; report robustness over the order envelope rather than silently choosing a favourable order.
- **Finite heterogeneous vehicles:** enforce capacity/type compatibility and vehicle availability; report rejected work and infeasibility only where an exact proof exists.
- **Causal recovery:** release each disturbance to the policy at the declared observation tick. Do not feed future evaluator truth into the policy.
- **Stability Pareto front:** after matching feasibility, compare recovery time, utilisation, vehicle travel, changed reservations, unaffected-work churn, and resource contention as separate axes.

Each extension must have its own explicit scenario family and should not be mixed into the paper's headline reproduction row.

## Proposed staged claim ladder

- **R0 — Corpus provenance:** source instance files, parameter tables, and raw outputs acquired and hash-locked.
- **R1 — Metric reproduction:** published Aur/Vt values computed independently from reconstructed schedules, with tolerance and units predeclared.
- **R2 — Algorithm reproduction:** PA/IA/GA/DE implemented or author-supplied implementations used with the published settings; report all 15 (or author-confirmed) runs per cell.
- **R3 — Fair matched-budget comparison:** new candidate methods receive the same immutable instances, disturbance histories, hardware profile, compute budget, stop conditions, and information access.
- **R4 — Held-out robustness:** unseen instance families and disturbance schedules, with paired per-episode deltas, uncertainty, worst-case/tail failures, and every timeout retained.
- **R5 — Industrial validation:** historical replay followed by shadow-mode APS/MES integration; only later consider a safety-owner-approved controlled operational evaluation.

No level may be skipped. A synthetic smoke test, a valid JSON ledger, or a zero-violation unary-resource schedule does not establish physical shipyard feasibility or state-of-the-art performance.
