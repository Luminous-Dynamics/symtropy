# Mobility Energy Policy Tournament: matched comparisons and SOTA benchmark protocol v0.1

**Benchmark:** MOBILITY-ENERGY-POLICY-TOURNAMENT-001 (#7410)
**Parent programme:** MOBILITY-ENERGY-RESILIENCE-001 (#6707)
**Implementation:** `symtropy-mobility-policy-tournament` Rust crate; paired outcome evaluator only, no optimizer/simulator
**Research status:** 25 November 2026 literature/industry snapshot

## 1. The paradigm shift

Stop asking which controller gets the highest reward, peak-shaving percentage or one-day cost. Ask which policy delivers the most **reliably qualified mobility and grid service** under the same physical scenario, evidence, time horizon, constraints, information budget and compute budget.

The benchmark has two independent planes:

1. **Assurance plane:** mission reserve, post-dispatch recovery, exclusive commitments, PCC and network limits, evidence integrity, cyber-integrity and authorization. This plane is lexicographic: a policy with a false-safe admission or hard-constraint violation is not rescued by a better reward, revenue or average mission count.
2. **Performance plane:** on-time and completed mobility service, queue delay, signed metered response, network-deliverable grid service, operating cost, degradation, renewable curtailment, site import, emissions proxy and decision latency.

Do not compress requested MW, instructed MW, measured PCC MW, counterfactual net kWh, network-deliverable MW and adequacy contribution into a single 'flexibility' number. They are different claims, and their source owners remain distinct.

## 2. What the executable evaluator enforces

The v0.1 Rust evaluator consumes completed policy-run summaries. It does not run the policy or simulate the battery. The scenario/replay owner must produce the run reports under the pinned input manifest.

### 2.1 Exact matched experiment

Every policy must have exactly one result for each scenario/replication pair. Every run carries an information-trace digest, an information-audit digest, a future-information access count and a nonanticipativity violation count. A nonanticipative-online run with any future-information access or nonanticipativity violation is flagged as an evidence-integrity hard failure. The audit receipts are supplied by the replay harness; the evaluator checks their digest shape and reported counts, but does not authenticate the audit producer or independently prove information flow. Across policies, the exogenous pair must match on:
- scenario ID, scenario family, independent cluster ID and replication ID;
- random seed, scenario digest and exogenous-input digest;
- evidence class;
- compute environment digest and per-run compute budget.

Policy family/version/configuration digest can and should differ. A missing pair, duplicate, seed mismatch, changed environment, changed compute budget or evidence-class mismatch rejects the tournament. Timed-out, failed and abstained runs remain in the denominator; every record must carry the realized horizon outcome, including safe-fallback behaviour.

### 2.2 Separate online and clairvoyant tracks

All policies in one tournament must use the same information model. Nonanticipative online policies cannot be mixed with a clairvoyant full-horizon upper bound. A clairvoyant bound may be useful in its own offline track, but is not an eligible deployed policy or a fair online regret comparator.

### 2.3 Safety and outcome metrics

The evaluator reports:
- on-time service and completed-by-horizon rates with their mission denominators;
- p95 queue-wait and decision-latency distributions;
- requested service, instructed response, signed metered response and network-deliverable energy separately;
- reserve and recovery shortfall, hard-gate violations by category, false-safe admission and false abstention;
- costs, degradation proxy, energy, curtailment and emissions proxy;
- empirical CVaR95 and maxima for mission-reserve and recovery shortfalls;
- outcome strata for every scenario family and independent cluster.

Positive metered response above the declared DC-network-deliverable cap is a visible hard network conflict, not an opportunity to clip the measurement. Negative metered response (increased grid import) stays negative.

### 2.4 Paired statistics, not pseudo-replication

Candidate-minus-reference deltas are calculated on identical scenario/replication pairs. Within each declared independent cluster (for example site-day or independently generated incident block), run-level deltas are averaged; bootstrap resampling then draws clusters, not individual 1-second or 15-minute intervals. The report records bootstrap seed, number of replicates, confidence level and cluster count. A checked work budget caps the product of independent clusters, bootstrap resamples, policy comparisons and six reported metrics at 100 million sample operations. With too few independent clusters, the confidence interval is withheld.

The intervals describe the empirical resampling design and the supplied cluster labels. They are not distribution-free field guarantees, nor do they establish independence where the underlying cluster design is wrong.

## 3. Industry and SOTA comparator map

### 3.1 Hierarchical day-ahead bidding and intraday tracking

Chen et al., *Electronics* 15(11), 2444 (3 June 2026), propose an aggregation control strategy for battery-swapping-station clusters. It combines demand forecasting, day-ahead cluster bidding, intraday rolling optimization at 15-minute intervals and real-time station recommendations at 10-second intervals. The paper uses operational data and co-optimizes demand allocation and charging/discharging strategies to track the declared grid trajectory.

Source: https://doi.org/10.3390/electronics15112444

**Matched comparator:** reproduce its day-ahead/intraday architecture and its published station/demand assumptions. Score tracking error, service rejection, station-level inventory, queueing and reserve restoration separately. Do not call a 15-minute schedule a 10-second frequency-response controller.

### 3.2 Two-stage robust capacity planning and adaptive dispatch

Wang et al., *Journal of Energy Storage* 152, Article 120628 (30 March 2026), combine PV, storage, charging and swapping under multi-source generation/load uncertainty. Their approach uses multi-bandwidth Γ-budget polyhedra plus α-cut fuzzy intervals, two-stage robust capacity configuration and intraday operation, and column-and-constraint generation after duality reformulation into an MILP. The paper reports, for its studied cases, up to 43.8% lower operating cost, 43.6% lower grid energy purchase and 43.3% fewer daily equivalent full cycles than deterministic methods.

Source: https://doi.org/10.1016/j.est.2026.120628

**Matched comparator:** reproduce the original uncertainty sets and test system before transfer. Compare capacity choices, worst-case feasibility, cost, grid purchase and degradation on the same uncertainty traces. The published percentage improvements are paper results, not a portable target or a result achieved by this repository.

### 3.3 Location–inventory–grid co-optimization

Ai et al., *Transportation Research Part E* 214, Article 104897 (October 2026), develop a continuous-approximation model integrating EV charging/swapping station location, inventory and distribution-grid interaction. It compares centralized and decentralized charging, each with and without frequency-regulation participation. Its analysis highlights a trade-off: frequency regulation can reduce cost and grid peak load while making cost more sensitive to inventory decisions.

Source: https://doi.org/10.1016/j.tre.2026.104897

**Matched comparator:** preserve spatial and inventory assumptions, then compare centralized versus decentralized depot layouts under identical mobility service levels, network cases and life-cycle boundaries. Report location/inventory sensitivity rather than one system-level cost alone.

### 3.4 Joint-chance distributionally robust planning

Xin et al., *Energy* 355, Article 141090 (15 July 2026), use a two-stage distributionally robust location/inventory model for commercial EV battery-swapping stations with joint chance constraints and moment-based ambiguity sets. They reformulate using a CVaR/SOCP approximation and solve with tailored outer approximation, then compare out-of-sample distributions including heavy-tailed demand and a commercial solver baseline.

Source: https://doi.org/10.1016/j.energy.2026.141090

**Matched comparator:** retain the full joint chance contract and the original ambiguity assumptions. Do not replace it with independent per-slot probabilities. Report joint service failure, worst depot, worst season and the conservatism gap.

### 3.5 Offline distributionally robust reinforcement learning

Du and Zhu, *Applied Sciences* 16(22), 11346 (16 November 2026), propose DR-SAC for offline heavy-truck battery-swapping-station scheduling under uncertain demand and time-of-use price. They use SOC-classified battery clusters and a KL-constrained robust MDP. In their high-demand test scenario, DR-SAC reports 78.9% peak shaving and 5.3 scarcity-violating periods/day; SAC reports 2.9, TD3 12.4 and DDPG 21.7. DDPG reaches 86.7% peak shaving but with 21.7 violations/day. The authors identify a need for a safety layer under high demand.

Source: https://doi.org/10.3390/app162211346

**Matched comparator:** reproduce its offline-data split, KL radius, battery scarcity limit and reward boundary before transfer. Put DR-SAC, SAC, TD3, MPC and robust optimization behind the exact same independent reserve/recovery/PCC/network shield. Report policy reward as a descriptive value only; the hard-gate plane remains separate.

### 3.6 Cyber-physical resilience

Ghelichpour et al., *Renewable Energy* 275, Article 126219 (1 November 2026), study real-time smart-grid operation with battery-swapping stations while modelling cyber-physical attacks, communication delay and network reconfiguration. Their framework integrates multi-objective optimization, LSTM prediction, fuzzy Monte Carlo scenarios and dynamic security assessment on a modified 118/124-bus study system.

Source: https://doi.org/10.1016/j.renene.2026.126219

**Matched comparator:** include false-data injection, stale/replayed telemetry, delayed instructions, changing topology, branch outages and communication partitions. Measure safe-mode latency, false-safe admissions, recovery and unserved mobility—not merely detection accuracy.

### 3.7 Online temporal and federated uncertainty

OMPB by Huang, Ma and Michailidis (UAI 2026 / PMLR 337:2244–2273) uses martingale PAC-Bayes certificates for time-series calibration under dependence and distribution shift, with predict-then-update timing. Reproduce the native benchmark and online protocol; the current #7290 temporal risk monitor is a simpler auditable baseline and does not claim the OMPB guarantee.

Paper: https://proceedings.mlr.press/v337/huang26b.html
Implementation: https://github.com/chenfeng-huang/OMPB-UAI-2026

FCPJS by Shi et al. (UAI 2026 / PMLR 337:6232–6260) addresses federated conformal calibration under heterogeneous joint shift. It reports valid marginal coverage up to its finite-sample/sketch approximation term and 12.64% predictive-efficiency improvement over the strongest baseline across four heterogeneous benchmarks. The current #7268 reducer is a deterministic histogram-summary baseline, not a reproduction of FCPJS.

Paper: https://proceedings.mlr.press/v337/shi26a.html

**Matched comparator:** use chronological site-held-out records, preserve local alarms and abstentions, and reproduce the published method in its own environment before claiming an improvement.

## 4. South African market and qualification comparison

The NTCSA Ancillary Services Technical Requirements for 2026/27–2030/31 specify reserves, system-restoration services, reactive power/voltage control and constrained generation. They state the reserve categories are exclusive: capacity assigned to one reserve category cannot simultaneously be used for another. Instantaneous reserve must be fully available within 10 seconds and sustained for at least 10 minutes.

Source: https://www.ntcsa.co.za/wp-content/uploads/2026/01/ASTR-2026_27-2030_31.pdf

The SAWEM Market Code Rev 2.3 (February 2026) defines Instantaneous Reserve as generating capacity or demand-side managed load that can respond fully within 10 seconds to a frequency drop and sustain that response for at least 10 minutes.

Source: https://www.ntcsa.co.za/wp-content/uploads/2026/02/Market-Code-v2.3.pdf

Eskom's Instantaneous Demand Response Programme has its own programme-specific conditions: load providers must respond fully within 6 seconds, sustain at least 10 minutes, and meet a 10 MW minimum load entry. It also specifies special metering and a limited event schedule. Do not misstate these programme thresholds as universal SAWEM requirements for every BESS or mobility fleet.

Source: https://www.eskom.co.za/distribution/instantaneous-demand-response-programme/

AEMO's final BESS Contingency FCAS Registration Guide V5.0 was published 2 September 2026, following consultation. AEMO says the updated guides clarify testing, simulation and assessment of capability; its MASS and FCAS Verification Tool separately define performance parameters and verification procedures. These are useful industry examples of product-specific technical qualification and verification, but they are not South African market rules.

Sources: https://www.aemo.com.au/consultations/current-and-closed-consultations/fcas-registration-guides-bess-wind-farms-and-solar-farms-consultation ; https://www.aemo.com.au/energy-systems/electricity/national-electricity-market-nem/system-operations/ancillary-services/market-ancillary-services-specification-and-fcas-verification-tool

## 5. Tournament protocol

### 5.1 Freeze the exogenous world

Each run manifest must freeze the scenario corpus, demand/mission arrivals, weather/temperature, PV generation, grid tariff and capacity, topology/contingencies, site load, asset/BMS/charger configuration, baseline, event timeline and decision-time evidence availability. Only policy configuration and policy-specific randomization may differ. Common random numbers should be used where stochastic methods permit.

### 5.2 Use a staged benchmark ladder

| Track | Required baselines | Primary decision question |
|---|---|---|
| Mobility-only charging | fixed-cap / charge-on-arrival / FCFS | Does grid participation encroach on mission service? |
| Self-consumption | PV + stationary ESS, no export/service | How much bill/peak reduction is available without external services? |
| Day-ahead | TOU deterministic MILP | What is the value of forecast-aware charging? |
| Intraday | rolling 15-minute optimization and MPC | How much does receding-horizon state update improve service/tracking? |
| Robust planning | two-stage RO, Γ-budget, Wasserstein DRO, joint-chance DRO | What is the cost/reliability/conservatism trade-off? |
| Learned proposal | DR-SAC, SAC, TD3, offline constrained RL | Does a learned policy generalize under common stress and held-out depots? |
| Assurance shield | every candidate plus identical deterministic gates | What unsafe proposals are rejected, and what is the opportunity cost? |
| Event reconciliation | response, PCC metrology, baseline/rebound and network reports | How much signed net service is supported after uncertainty and recovery? |

Keep planning, intraday scheduling, real-time tracking and post-event measurement as separate tracks. A day-ahead schedule does not compete directly with a sub-second controller or a settlement/M&V estimator.

### 5.3 Data splits and leakage controls

- Hold out complete depots/sites, asset/BMS configurations, season/temperature regimes and independent incident clusters.
- Use chronological training/validation/test windows for every forecast-based policy; freeze normalization and hyperparameter selection to training/validation.
- Use no future arrivals, weather, network status or telemetry that was unavailable at the decision timestamp.
- Keep offline clairvoyant schedules in a separate upper-bound track; online policy regret is measured against a nonanticipative comparator, never the clairvoyant result.
- Use the same grid model, PCC, network contingency set and recovery deadline for all policies.
- Preserve every failed/timeout/abstained scenario in the outcome corpus.

### 5.4 Required result tables

Publish per policy and per scenario family/cluster:
- on-time missions / committed missions; completed / unserved / in-progress-at-horizon;
- p50/p95/p99 queue wait, response latency and compute latency;
- mission reserve shortfall, recovery shortfall, reserve restoration deadline misses;
- requested, instructed, signed metered and network-deliverable grid response;
- energy purchase, site peak, renewable curtailment, storage throughput, degradation cost and total cost;
- false-safe admissions, false abstentions, hard-gate violations and shield intervention rate;
- empirical CVaR95/worst-case reserve and recovery shortfall;
- paired candidate-minus-reference effect, independent cluster count and cluster-bootstrap interval.

Do not report only reward, cost, peak shaving or average service. A policy that produces more peak reduction but incurs reserve failures is not better.

## 6. Benchmark targets

These are engineering acceptance targets for this programme, not achieved performance claims.

### Contract and safety

- 100% pair completeness: every declared policy covers exactly the same scenario/replication keys.
- 100% common seed, exogenous input, evidence class, compute environment and compute budget identity.
- 0 silently discarded timeout, failed, abstained or invalid runs.
- 0 false-safe admissions on frozen independent hard-negative controls.
- 0 mission-reserve, recovery, authorization, double-commitment or network violations for a policy labelled admissible.
- 100% exact agreement with an independent hand-computed reference on tiny matched fixtures.
- 100% deterministic scorecard digest across policy/run/violation ordering permutations.

### Performance and statistics

- First establish a mobility-only non-inferiority floor for on-time service and reserve safety; do not set an arbitrary universal service-rate threshold before physical baseline data exists.
- Measure a Pareto frontier across on-time mission service, net network-deliverable MWh, cost per successful mission, cost per verified grid-service kWh, degradation and response latency.
- Report p50/p95/p99 and empirical CVaR95 for wait, shortfall and cost; do not treat individual intervals as independent observations.
- Require at least 30 independent site-day/incident clusters before treating 95% cluster-bootstrap intervals as a stable operational comparison; smaller studies report descriptive deltas and withhold inferential claims.
- Compare p95/p99 decision latency under a fixed published compute environment/budget; report timeouts as failures, not as omitted records.
- Report the assurance shield's intervention rate and economic opportunity cost independently. A low intervention rate is not automatically good if false-safe admission increases.

### What would count as beating SOTA?

A credible result is a Pareto improvement over a reproduced comparator on a matched held-out corpus: at least equal on-time mobility service and hard-gate safety, greater positive network-deliverable metered service or lower cost per successful mission, no deterioration in worst-stratum/CVaR95 risk, and acceptable compute latency. Any claimed advantage must remain after the same deterministic assurance shield is applied to all policies.

Do not claim superiority by comparing our synthetic 4-cluster fixture to a paper's real station dataset or IEEE test feeder. First reproduce each paper natively; then run a separate common Luminous mobility-depot corpus with transparent mapping assumptions.

## 7. Immediate next increments

1. Complete exact-head formatting, Rust tests, Clippy and the CLI known-answer workflow.
2. Export machine-readable result artifacts from #6803/#6809 and #7313 and response/baseline/metrology reports from #3649/#3772/#3807; preserve exact input/configuration digests.
3. Build the first 2–4-resource, 8–24-slot deterministic exact small-instance optimizer/independent enumeration reference; keep it proposal-only.
4. Reproduce Chen et al. cluster scheduling and Wang et al. robust capacity/dispatch methods on their native assumptions.
5. Add DR-SAC/SAC/TD3 on matched offline replay data; report scarcity-violation and service tails, not just profit.
6. Add false-data injection, stale topology, communication delay, charger/ESS outage, repeated shocks and correlated weather/demand incidents.
7. Start a measured stationary-ESS/PCC pilot before propulsion-pack grid service. Collect calibration traceability, high-resolution PCC response, baseline/recovery and operator qualification evidence.

## 8. Claim ceiling

The v0.1 evaluator compares the supplied finite policy outcomes only. It does not implement a scheduling policy, validate the simulator, authenticate report producers, establish causal savings, certify battery or inverter performance, grant SAWEM/Eskom/AEMO participation, authorize export/dispatch, prove AC feasibility, or compute ELCC. Paper and synthetic evidence remain distinct from measured replay and physical trial.
