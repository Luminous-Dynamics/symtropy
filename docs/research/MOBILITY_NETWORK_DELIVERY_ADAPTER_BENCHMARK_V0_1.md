# Mobility Network Delivery Adapter and Benchmark Programme v0.1

**Programme:** MOBILITY-NETWORK-DELIVERY-ADAPTER-001 (#7293)
**Parent mobility assurance:** #6802, #6803, #6809, #6829, #6837
**Network solver:** GRID-LOC-DELIV-001 (#3826), implementation PR #3827
**Research cutoff:** 24 November 2026
**Implementation status:** read-only Rust adapter, exact-PCC mapping, strict input schema, synthetic three-bus fixture, JSON CLI and dedicated workflow authored; exact-head compile/test/lint is pending.

## 1. The field-level problem

Battery-enabled mobility fleets are simultaneously service systems and grid-connected flexible resources. Their available flexibility is not a scalar leftover-energy number. A credible offer is a time-indexed trajectory whose admissibility depends on the exact asset/configuration, mission obligations, inventory reserve, charge queue, recovery window, unique commitment state, PCC baseline and metering boundary, network topology, binding constraints and required contingency set.

Current toolchains often optimize one layer at a time: battery-swapping station economics, EV charging demand, grid power flow, reserve adequacy, or dispatch control. A local optimizer may propose a profitable response that is not network-deliverable at the PCC bus, fails after an N-1 outage, cannot recover before the next mobility service, or has already been promised elsewhere.

The integration gap is not another optimizer. It is a reproducible, fail-closed composition boundary linking the trajectory and its upstream evidence to a locational network calculation while preserving each layer's ownership and claim ceiling.

## 2. Paradigm shifts

### 2.1 From spare kWh to an evidence-qualified feasible trajectory

Represent flexibility as a trajectory over time, direction, ramp, duration, energy, asset, PCC, uncertainty and topology. The candidate is admitted only when the intersection of all per-resource reserve/recovery constraints, unique commitment state, PCC envelope and network constraints remains non-empty over the full offered horizon.

### 2.2 From one firm-MW label to three non-interchangeable quantities

1. **Site-requested or site-metered response:** what the resource offer requests or the PCC meter later measures.
2. **Network-deliverable response:** what the exact topology, injection pattern, branch ratings and contractually required contingencies permit.
3. **Adequacy contribution:** how the portfolio changes probabilistic loss-of-load, expected unserved energy, scarcity or ELCC metrics.

The v0.1 adapter computes only the second quantity, retains the first as requested offer volume and marks actual post-event metered delivery unassessed. It does not compute the third.

### 2.3 From optimizer authority to optimizer plus independent assurance

Allow MILP, MPC, robust optimization or reinforcement learning to propose trajectories. A deterministic evidence and physics gate independently screens each proposal. The optimizer cannot edit its own safety margins, receipt status, PCC mapping, topology digest or reserve state. Rejected proposals remain valuable learning data but cannot become dispatch.

### 2.4 From static qualification to time- and version-bounded applicability

Forecast/calibration evidence, reserve receipts, topology data, pack/charger configuration and commitment leases all have issue times, validity intervals and exact version digests. A model update, pack replacement, BMS firmware change, site reconfiguration or topology change begins a new applicability epoch; yesterday's evidence is not silently inherited.

### 2.5 From single-shock reliability to recoverability under repeated shocks

An offer that passes its response window but leaves the pack or depot unable to restore required reserve before the next mission is not viable flexibility. The primary objective must include reserve restoration, cumulative degradation, queue congestion and common-mode disturbance exposure.

### 2.6 From point forecasts to decision-impact and joint-risk contracts

Use calibrated uncertainty and prequential update rules, but score them at the operational decision boundary: false-safe admission, false abstention, recovery shortfall, service success and network violation. Report fleet averages and worst strata separately; a high global coverage number must not hide an unsafe pack/site/temperature group.

## 3. Implemented v0.1 adapter boundary

The adapter crate depends on the existing symtropy-dc-network-screen implementation. It constructs the existing DC model/interval types and calls evaluate_dc_network_deliverability; it does not implement a second susceptance-matrix solver, PTDF/LODF engine or AC model.

Mandatory input bindings:
- exact offer ID and commitment epoch;
- exact site/PCC/PCC bus and storage asset/configuration;
- exact baseline and offered trajectory digests;
- decision cutoff and complete trajectory horizon;
- frozen network model/topology IDs and digests, explicit MVA base and branch ratings;
- baseline non-slack injections per time interval and requested response at the pinned PCC only;
- exactly one mobility-flexibility, reserve-recovery, commitment-lease, commitment-journal and PCC-metrology receipt;
- explicit N-1 contingency list and policy.

Receipt validation enforces identity, exact gate set, uniqueness of receipt IDs/references/digests, valid SHA-256 digest shape, evidence observation times, validity through the full offer horizon, asset/configuration scope, exact network/topology binding on the mobility-flexibility receipt, and trace-pair binding on the PCC-metrology receipt. Digest shape is not signature verification or producer authentication. Unknown gates hold the offer; failed gates reject it; neither path invokes the network screen.

The DC report is preserved as a nested report. The adapter separately reports requested MWh, maximum DC-network-deliverable MWh and the remaining undeliverable/curtailed MWh. It keeps actual meter delivery unassessed and leaves AC feasibility, adequacy, market eligibility, interconnection approval, field qualification and dispatch authority false.

## 4. Frozen synthetic known answer

The fixture uses the existing hand-computed three-bus triangle from #3826/#3827:

| Quantity | Frozen expected value |
|---|---:|
| PCC B requested response | 30 MW for one hour |
| Site-requested energy | 30 MWh |
| Maximum DC-network-deliverable response | 17.5 MW / 17.5 MWh |
| Undeliverable / curtailed remainder | 12.5 MWh |
| Full requested trajectory network-deliverable | false |
| Mandatory upstream gate receipts | 5, all synthetic and marked passed |
| Actual post-event metered response assessed | false |
| Adequacy / AC / market / interconnection / field / dispatch authority | all false |

The 17.5 MW fraction is the linear DC branch-limit result already implemented by #3827 for this synthetic topology. The adapter verifies composition and conservation; it does not claim an optimizer, operating policy or field performance improvement.

## 5. SOTA and industry comparison

### 5.1 Distributionally robust battery swapping and joint chance constraints

A 2026 Energy paper on commercial EV battery-swapping station location/inventory planning formulates a two-stage distributionally robust optimization with joint chance constraints, a moment-based ambiguity set, SOCP reformulation and an outer-approximation algorithm. It compares out-of-sample demand distributions and includes a commercial solver comparator. This is a stronger planning benchmark than deterministic single-scenario arithmetic. Our matched tournament should use the same demand scenarios and service constraints, then additionally bind each candidate to the exact asset/PCC/topology evidence and recovery gates.

Source: https://doi.org/10.1016/j.energy.2026.141090

### 5.2 Distributionally robust reinforcement learning

Du and Zhu (16 November 2026) propose DR-SAC for electric heavy-truck battery-swapping station scheduling under demand and TOU-price uncertainty. In their high-demand scenario, DR-SAC reports 78.9% peak shaving and 5.3 scarcity-violating periods/day; SAC reports 2.9, while TD3 and DDPG report 12.4 and 21.7. DDPG reaches higher peak shaving (86.7%) but has more violations. The authors explicitly identify a need for a safety layer under high demand. Their work is based on one mining-area station and its own tariffs/data; these values are not transferable UAV results.

Source: https://doi.org/10.3390/app162211346

### 5.3 Cyber-physical resilience and delay

Ghelichpour et al. (1 November 2026) address real-time smart-grid operational planning with EV swapping stations, combining multi-objective optimization and an LSTM with fuzzy Monte Carlo scenarios, communication delays, cyber-physical attacks and reconfiguration. The adapter's next stress suite should reproduce matched delay and false-data scenarios rather than test only static topology and ideal communications.

Source: https://doi.org/10.1016/j.renene.2026.126219

### 5.4 Full AC power-flow co-optimization

A November 2026 study on renewable-integrated EV charging and battery swapping describes a distributionally robust multi-objective framework with integrated EVCS/BSS scheduling under full AC power-flow constraints. This is an essential comparator for the next validation stage: DC screening is a useful, auditable active-power prefilter, but it is not an AC-feasibility certificate. A matched AC solver must receive exactly the same topology, injections, ratings and operating point.

Source: https://doi.org/10.1016/j.esr.2026.102377

### 5.5 Online temporal calibration and federated uncertainty

Huang, Ma and Michailidis (UAI 2026) present OMPB, a model-agnostic online martingale PAC-Bayes calibration framework. It uses predictable variation, a source-risk anchor, posterior-shift penalty, time-adaptive mismatch and a gated residual Bayesian head. Forecasts are predict-then-update: each outcome is only used after it becomes available. Their stronger statistical guarantee and learnable correction should be reproduced on their native benchmark before comparison; our v0.1 prequential monitor is an auditable, smaller baseline and does not claim OMPB equivalence.

Paper: https://proceedings.mlr.press/v337/huang26b.html
Official implementation: https://github.com/chenfeng-huang/OMPB-UAI-2026

Shi et al. (UAI 2026) present FCPJS for federated joint shift. A server sketches global nonconformity distributions while each client calibrates locally with importance weighting. The paper reports marginal coverage up to its finite-sample/sketch approximation term and 12.64% predictive-efficiency improvement over its strongest baseline across four heterogeneous benchmarks. Our current fixed-bin federated reducer is a baseline only. The next increment should reproduce FCPJS natively and preserve local alarms and abstentions when a fleet-level summary is produced.

Paper: https://proceedings.mlr.press/v337/shi26a.html

## 6. South African service and procurement context

NTCSA's 2026/27–2030/31 Ancillary Services Technical Requirements identify reserves, system-restoration services, reactive power/voltage control and constrained generation as service categories. The adapter should not treat a generic BESS/MW value as an eligible product: each service needs its own technical response, duration, recovery and location requirements.

Source: https://www.ntcsa.co.za/wp-content/uploads/2026/01/ASTR-2026_27-2030_31.pdf

The February 2026 SAWEM Market Code v2.3 defines regulating reserve as generating capacity or demand-side resource able to begin responding to AGC instructions within 10 seconds and be fully activated within 10 minutes. This is a product-definition comparator, not proof that this mobility offer is a qualifying market resource.

Source: https://www.ntcsa.co.za/wp-content/uploads/2026/02/Market-Code-v2.3.pdf

On 7 October 2026, the South African Government announced a Section 34 procurement direction prioritising 4,600 MW BESS and 5,000 MW gas-to-power; Notice 8021 was published on 14 October 2026. This is a procurement direction for new capacity, not an entitlement or market approval for a mobility fleet. It strengthens the case for a future South African evidence campaign, but network data, interconnection rights, market-participant qualification and operator approvals remain required.

Sources: https://www.gov.za/news/media-statements/minister-kgosientsho-ramokgopa-prioritises-battery-storage-and-gas-power ; https://www.acts.co.za/electricity-regulation-act-2006/n8021_notice_no__8021_of_2026

## 7. Benchmark tournament

Every strategy must receive the same forecast snapshots, workload/mission schedule, battery/charger configurations, outage sequence, PCC metering boundary, topology, contingency set, tariffs and compute budget. Hold out whole sites, packs/configurations, weather periods and incident blocks; do not random-split temporal records.

| Strategy family | Required matched comparator | What must be measured |
|---|---|---|
| Fixed mobility-only reserve | Conservative static buffer / charge-on-arrival | Mission success, service latency, unused reserve |
| Deterministic grid scheduling | TOU/MILP day-ahead dispatch | Cost, peak reduction, feasible MWh, solve time |
| Rolling receding-horizon control | MPC with identical horizon and forecasts | Tracking, constraint violations, re-optimization latency |
| Stochastic planning | Scenario stochastic program | Expected cost, tail cost, scenario feasibility |
| Distributionally robust planning | Two-stage joint-chance DRO/SOCP/outer approximation | Out-of-sample reliability and conservatism gap |
| Learned proposal | DR-SAC / SAC / TD3 with same state/action interface | Reward, service, worst-scenario violations, shield intervention rate |
| Resilient operation | Delay/FDA/reconfiguration stress method | Detection, degraded operation and recovery |
| Network screening | Existing DC screen and independent AC solver | DC/AC deviation, binding constraints, N-1 and voltage violations |
| Statistical assurance | OMPB, FCPJS, rolling conformal, adaptive conformal, CUSUM | Coverage, interval width, calibration delay, group/worst-stratum risk |

### 7.1 Hard-gate targets

- 100% agreement with the existing DC solver on identical frozen inputs.
- Zero acceptance for missing, failed, unknown, stale, duplicated or mismatched upstream gate evidence.
- Zero flexibility action at any bus other than the exact PCC bus.
- Zero topology/contingency omissions when N-1 is a contract requirement.
- 100% requested-energy conservation: requested = network-deliverable + undeliverable/curtailed, within declared numerical tolerance.
- 100% deterministic canonical report under receipt, branch, bus and contingency order permutations.
- 0 false-safe admission across all frozen hard negative controls.
- 0 promotion of DC output to AC feasibility, ELCC, market eligibility or dispatch permission.

### 7.2 Performance and economics targets

- Report p50/p95/p99 adapter overhead separately from DC solver runtime at 1, 8, 24 and 96 intervals.
- Compare solve/decision latency at a fixed declared CPU, memory, horizon and contingency budget.
- Report net delivered MWh, not nameplate MW or requested MWh.
- Report cost per reliable net delivered MWh, including degradation, reserve opportunity cost, charging energy, market/imbalance charges and service-failure penalty.
- Report QoS/mission service delivered, false-safe admission, false abstention, recovery shortfall, reserve encroachment and common-mode joint-delivery shortfall.
- For the AC stage, report voltage margin, reactive power, losses and line/transformer overloads; DC-only results cannot receive AC credit.
- For optimizer comparisons, report the price of the assurance shield: proposals rejected, deliverable fraction, opportunity cost and remaining service.

Initial engineering target: p99 adapter overhead below 10 ms for 8 intervals and 4 contingencies on a declared reference runner. This is a research target, not a measured result. Larger horizons must publish complexity and p99 measurements rather than extrapolate.

### 7.3 What it would mean to beat SOTA

Do not define success as a higher reward or peak-shaving percentage when constraints are violated. A credible win is a Pareto improvement under the same hard gates: equal or better net delivered MWh and service quality, lower total cost per reliable delivered MWh, tighter or equally valid uncertainty, lower or equal false-safe risk, and bounded decision latency. At least one axis must improve materially without degrading another safety-critical axis. Results must hold on held-out sites/configurations and repeated-shock/cyber/topology tests.

## 8. Next increments

1. Complete exact-head format, test and Clippy verification for this adapter; publish run IDs and logs.
2. Add the mobility adapter as a public interface consumed by a higher-level portfolio optimizer, with no authority bypass.
3. Add a measured PCC response receipt after a real dispatch trial; distinguish requested, scheduled, instructed and measured response.
4. Reproduce OMPB, FCPJS and the 2026 joint-chance DRO/DR-SAC comparators in matched native environments.
5. Add independent AC power-flow comparison using identical network data and limits; then extend to feeder constraints and RMS/EMT/HIL as distinct claim classes.
6. Add explicit delay, false-data injection, stale topology, communication loss, station outage, pack/charger failure and common-weather shocks.
7. Add PRAS/ELCC only as a separate probabilistic adequacy layer consuming qualified, time- and location-specific trajectories.
8. Build a South African pilot data request around NTCSA product requirements, a measured site PCC, qualified stationary ESS first, and a separately permissioned mobility-pack experiment.

## 9. Claim ceiling

The adapter composes upstream receipts and performs deterministic DC screening only. It is not a dispatch controller, electrical protection system, AC power-flow solver, utility planning model, market participant, ELCC calculation, interconnection approval or field qualification. A positive DC result is only one input to subsequent independent technical and institutional approvals.