# AERO-HAPS-EDGE-PLACE-001 — HAPS edge placement and co-scheduling

Status: synthetic-first research specification v0.1.
Parent network allocation: [PR #5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948).
Tracking issue: [#5989](https://github.com/Luminous-Dynamics/symtropy/issues/5989).
Claim ceiling: deterministic placement arithmetic only.

## Why it matters

HAPS are becoming compute-bearing network infrastructure, not only relays. Sceye / SoftBank's ST1 trial reported an onboard mobile core and server, with a 68 ms average round-trip processing response for the demonstrated service path. This is a source-attributed average, not a p95/p99 SLA or measured aircraft thermal/power envelope. [Sceye ST1 trial](https://sceye.com/press-releases/sceye-and-softbank-complete-stratospheric-connectivity-demonstration-in-japan/).

A direct HAPS-related baseline is Nguyen et al., ICT Express, online 26 October 2026: MATD3 for HAPS-assisted multi-UAV edge computing. It jointly selects UAV trajectories, offloading ratios and compute priorities; a decision map derives feasible compute/bandwidth allocations. The paper reports simulated mean cost 15.4-16.9% below MAPPO and 2.3-7.5% below MADDPG. These are paper-specific simulation results, not measured HAPS hardware results. [DOI 10.1016/j.icte.2026.10.009](https://doi.org/10.1016/j.icte.2026.10.009).

Other methods to reproduce include hybrid-action MADRL with task migration ([10.1109/TMC.2026.3712259](https://doi.org/10.1109/TMC.2026.3712259)), priority-aware MAPPO plus bipartite matching ([10.1016/j.comcom.2026.108684](https://doi.org/10.1016/j.comcom.2026.108684)), and HAPS-LEO MAPPO with exact-potential-game coordination ([10.1109/JIOT.2025.3629556](https://doi.org/10.1109/JIOT.2025.3629556)). ITU-T Y.3212 explicitly includes HAP access-network, control-plane, user-plane and application functions within fixed/mobile/satellite convergence: [ITU-T Y.3212](https://www.itu.int/ITU-T/recommendations/rec.aspx?rec=16109).

## Paradigm shift

Treat compute placement as a cyber-physical resource decision:

task arrival -> placement -> network path -> queue/compute -> result delivery -> electrical power / waste heat / reserve -> fallback and handover.

HAPS compute can save latency and backhaul, but it converts electrical load into heat in a rarefied environment and competes with propulsion, avionics, thermal management and night-energy reserve. Drone-local execution may be safest for a control loop. A terrestrial gateway may preserve aircraft reserve. Remote cloud may be acceptable for flexible tasks but not for a safety-critical loop. The scheduler should learn when not to offload.

## Machine-readable artifacts

- [haps_edge_placement_case_v1.schema.json](schemas/haps_edge_placement_case_v1.schema.json)
- [synthetic_haps_edge_placement_case_v1.json](examples/synthetic_haps_edge_placement_case_v1.json)

The contract declares workload criticality, deadline and latency basis, compute demand, candidate node, response time, node power/heat, HAPS network power/heat and HAPS backhaul.

## Synthetic exact-oracle result

Three workloads: emergency alert fusion (0.05 TOPS, 250 ms), drone control/telemetry (0.5 TOPS, 100 ms control-loop round trip), and wildfire video analytics (2 TOPS, 1,000 ms). Node limits: HAPS 2.2 TOPS / 90 W / 70 W heat; drone 0.6 TOPS / 45 W / 40 W heat; gateway 10 TOPS / 70 W / 60 W heat; cloud 100 TOPS / 150 W / 120 W heat. HAPS backhaul cap is 10 Mbps, incremental power cap 90 W and heat cap 70 W.

All 3 x 4 x 3 = 36 candidate assignments were enumerated; 11 are feasible.

| Policy | Alert | Drone control | Video analytics | System power | HAPS power | HAPS heat |
|---|---|---|---|---:|---:|---:|
| Lowest-latency feasible | HAPS | Drone | HAPS | 113 W | 73 W | 65 W |
| Minimum-system-power feasible | HAPS | HAPS | Gateway | 93 W | 58 W | 49 W |
| All-on-HAPS negative control | HAPS | HAPS | HAPS | 110 W | 110 W | 99 W; infeasible |

The minimum-power placement is 17.7% lower total system power and 20.5% lower HAPS incremental power than the lowest-latency feasible choice, while all three tasks meet their synthetic deadlines. Its priority-weighted response is higher (1,225 vs 695 ms), so this is a Pareto trade, not a universal winner. All-on-HAPS is rejected for exceeding compute, power and heat budgets.

These figures are independently replayed synthetic arithmetic, not measured device/platform power, physical cooling, a radio model, or an achieved HAPS performance result.

## Matched methods and targets

Compare fixed HAPS/gateway/drone placement, earliest-deadline-first, priority greedy, minimum-energy feasible, exact enumeration/ILP for small cases, MAPPO, MADDPG, MATD3, hybrid-action MADRL with migration, HAPS-LEO MAPPO and robust MPC. Freeze workload traces, capacities, path latencies, model fidelity, training data/steps, inference budget and failure accounting.

Targets, not achievements: reproduce MATD3/MAPPO/MADDPG first; at equal deadline compliance target >=10% lower system energy or latency-energy cost; at equal HAPS Wh/thermal headroom target >=10% more priority-weighted completions; zero unhandled safety-critical deadline misses or platform power/heat/reserve violations in runs marked feasible. Report p50/p95/p99 latency, thermal headroom, HAPS Wh, backhaul bytes and failover behavior.

## Ownership

Compose [network allocation PR #5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948), [HAPS qualification PR #5910](https://github.com/Luminous-Dynamics/symtropy/pull/5910), battery qualification [#3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903) and mission electro-thermal [#11411](https://github.com/Luminous-Dynamics/symthaea/issues/11411). No duplicate thermal or physics solver is created.

No radio transmission, payload installation, drone command authority, flight control, airspace operation or physical test is authorized.
