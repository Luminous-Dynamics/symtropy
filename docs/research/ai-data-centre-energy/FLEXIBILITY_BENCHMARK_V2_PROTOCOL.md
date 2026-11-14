# AI Data-Centre Flexibility Benchmark v2

**Protocol:** AI-DC-FLEX-BENCH-002  
**Version:** 0.1  
**Parent issue:** #3776  
**Programme:** #3435  
**Purpose:** benchmark useful, measurement-qualified and locationally deliverable flexibility against field, workload-efficiency, storage and grid-system baselines.

## 1. Why v2 is different

The field does not have one meaningful data-centre flexibility score. There are several different claims:

1. A compute stack can complete the same useful work more efficiently.
2. Workloads can be deferred or migrated without violating service.
3. A command produces a signed, sustained and measured change at the facility PCC.
4. The site has the physical reserve to deliver repeatedly without compromising UPS, cooling, thermal, water or critical-load constraints.
5. The response is deliverable at the relevant grid location and improves a defined system risk or stability metric.
6. The value of the response exceeds the opportunity cost of delayed/foregone compute and the cost of competing flexibility assets.

These claims require different tests and evidence classes. v2 therefore uses a **proof-carrying flexibility vector**, not a scalar score. Hard failures in safety, SLA, metrology or deliverability remain non-compensable.

## 2. Published baselines and how to use them

### Field envelope: Nature Energy

Colangelo et al., DOI 10.1038/s41560-025-01927-1, field-demonstrated software workload orchestration on a 256-GPU cluster: 25% power reduction for 3 hours while maintaining the study's AI QoS guarantees, without hardware modification or storage.

**Benchmark use:** replicate the published workload, hardware class, measurement boundary, duration and QoS contract as closely as the public experiment archive permits. A reproduction is comparable only when the original conditions and metrics are actually matched. The 30%-for-3-hours figure is our stretch target, not a claim against a different workload or GPU platform.

### Trace flexibility: Alibaba LAD-Flex

Caprara et al., DOI 10.1016/j.ijepes.2026.111940, used more than one million Alibaba task records. Their trace-based one-week study reports up to 22% load deferral during flexibility windows and more than 20% of estimated GPU-side power associated with deferrable tasks.

**Benchmark use:** recreate the published task filters, latency groups, duration assumptions, notification windows and flexibility windows before testing a new scheduler. Keep it labelled trace/model evidence, not a field grid-service demonstration. Score deadline misses, queue stability, completed work and backlog-clearing after the event.

### Cross-site response: ACM Sustainability Week field trial

The 22 June 2026 paper, DOI 10.1145/3765611.3815063, measured workload migration across operational data centres. The evaluated strictest response class was only met by pre-launched read-only patterns in both directions; multiple patterns met minute-level products. The paper does not establish complete market settlement, baseline, telemetry or product accreditation.

**Benchmark use:** preserve the distinction between replication, remote access and on-demand copy; pre-launch versus launch-at-command; read-only versus read-write. Do not generalize a pre-launched read-only response time to stateful read-write workloads.

### Efficiency: MLPerf Power

MLPerf Power reports tokens/Joule and samples/Joule across an instrumentation methodology that adapts across hardware scales. Its 2026 paper reports 1,841 reproducible submissions from 60 systems (DOI 10.1109/MM.2026.3700126).

**Benchmark use:** compare only within matched hardware, model, precision, quality/accuracy, workload and measurement boundaries. MLPerf efficiency is not a dispatch-response or grid-capacity benchmark. Publish both tokens/J and completed-useful-work/J, and never trade away answer quality or service SLOs to improve efficiency.

### Grid adequacy: PRAS 0.9.0

Use NLR PRAS as an independent sequential Monte Carlo multi-region adequacy engine. The pinned v0.1 harness currently has 42/42 exact-head assertions passing for synthetic known-truth cases; it does not reproduce NTCSA or a real site model. Keep PRAS LOLE event-periods, LOLD event-days, EUE and energy-CVaR distinct. Pin PRAS 0.9.0's strict-above-quantile empirical CVaR convention in cross-model comparisons.

### Measurement chain: IEC/IEEE 61869-21:2025 and GUM

IEC/IEEE 61869-21:2025 specifies uncertainty evaluation in instrument-transformer accuracy testing, including on-site testing. It does not alone qualify the complete installed PCC chain, meter integration, time alignment or data path. Use GUM-compatible uncertainty budgets and distribution propagation where the linear approximation is not adequate. Do not claim standards certification from software fixture tests.

### Grid and economic context: IEA 4E and IEA

IEA 4E's July 2026 report distinguishes market-serving, grid-serving and system-serving flexibility. The IEA's October 2026 report surveys 34 policy jurisdictions representing around 90% of global data-centre capacity. Its cited comparison places compute rental at approximately USD 20–30 billion per GW-year, around 100 times the equivalent battery capacity cost. This comparison is not a universal price curve, but it invalidates the assumption that curtailing compute is automatically cheaper than a battery.

**Benchmark use:** compute opportunity cost, service harm and revenue loss explicitly. Compare workload flexibility against BESS, UPS reserve, thermal storage, cooling shift and network upgrades on a common net-service basis.

## 3. Candidate strategies to compare

Every strategy must use the same frozen workload trace, service contract, hardware, forecast issue times, grid event, measurement boundary, scenario distribution, response window and computational budget.

| Strategy | Minimum comparator contract |
|---|---|
| Static / TDP | TDP-derived power and static schedule; negative control, never ground truth |
| FIFO / EDF | Queue policy with exact job deadlines and work obligations |
| GPU cap | Clock/power cap only, with same-output QoS and energy measurement |
| Deferral | LAD-Flex-style latency-aware deferral with queue/backlog model |
| Migration | Replication, remote access, on-demand copy, launch and read/write strata |
| BESS / UPS | SOC, reserve, power/energy limits, efficiency, losses and degradation |
| Cooling / thermal | Electro-thermal state, coolant limits, water, auxiliary power and recovery |
| Rule-based hybrid | Conventional priority/fallback control using identical constraints |
| Deterministic optimization | MILP and rolling-horizon MPC |
| Stochastic optimization | Sample-average CVaR / scenario MPC |
| Distributionally robust | Moment-based or other declared ambiguity set; robust-box sensitivity |
| Symtropy | Hierarchical co-optimizer; local controls retain veto and safe fallback |

Optimizer comparisons must report wall time, CPU/GPU-hours, memory, scenario count and solver tolerance. Compare at equal compute budgets and publish cost/reliability Pareto frontiers.

## 4. Frozen splits and data provenance

Split by time, site and hardware, not only random rows:
- training/tuning;
- validation and model selection;
- held-out time windows;
- held-out site;
- held-out accelerator/hardware family;
- stress and out-of-domain scenarios.

Bind each dataset and split to an immutable digest, source licence, preprocessing script digest and claim class. A source-published trace is not the same as a measurement made by Symtropy. Keep measured, source-reported, trace-replay, simulated, reconstructed, HIL, laboratory and field results distinct.

No workload session, site, event or time window may leak across splits. Do not fit a power model or choose a scheduler using the held-out evaluation interval.

### High-resolution accelerator/node trace admission card

The 2026 Scientific Data trace publication (DOI 10.1038/s41597-026-07496-6) describes 32 H100/B200 eight-GPU node sessions and 40 RTX 3060 sessions, with more than 1.8 million samples. Node-scale recording is 20 ms (50 Hz), single-machine recording is nominally 100 ms, and the sessions are 15 minutes long.

Its use in this programme is limited to per-GPU and GPU-sum node workload dynamics:
- the article states node-scale CPU power and temperature columns are empty because the virtualized host did not expose those sensors;
- node power is the sum of GPU powers, not facility AC input or a PCC measurement;
- GPU temperature is present, but ambient conditions and the facility cooling configuration are not;
- correlated oscillations among eight GPUs executing a shared training job inform intra-node common-mode modeling; they do not establish cross-site fleet correlation;
- 15-minute samples do not demonstrate a 3-hour flexibility service or post-event queue/thermal recovery.

The data record has a license ambiguity that must be resolved before reuse: the Figshare DOI landing page displays CC BY 4.0, while the companion GitHub repository README points to CC BY-NC-ND 4.0. Until the licensor confirms the applicable data license and permitted use, mark the source `UNRESOLVED_CONFLICT`; do not redistribute raw files, publish adapted dataset artifacts, or infer commercial-use permission. The source software-code license and dataset license must be treated separately.

Do not apply zero-order-hold resampling by default merely because the paper used it to align its two monitoring tools for a measurement comparison. Preserve native timestamps and intervals first; compare at a common resolution only under a declared integration/aggregation model that conserves energy and reports induced uncertainty. The paper's timing evaluation is source-reported validation, not a calibration certificate for any Symtropy meter.
## 5. Required metric vector

### Compute and service
- completed useful work and exact request/job obligations;
- request success, output correctness, deadline misses and cancellations;
- p50/p95/p99/p99.9 TTFT and completion latency when sample counts support the tail;
- queue growth, queue stability and post-event backlog-clearing time;
- tokens/J, samples/J and energy per completed useful unit at matched quality.

### PCC response
- command-to-first-response, time-to-90% and time-to-stabilized response;
- signed delivered MW, ramp, tracking error and sustained-delivery fraction;
- failed-call probability, response availability and repeatability;
- gross reduction MWh, gross rebound MWh, net MWh and post-event peak;
- measurement uncertainty divided by product threshold, interval lower-bound eligibility and clock alignment.

### Physical plant
- cooling/auxiliary power, coolant and thermal state, water consumption;
- maximum temperature and thermal-limit violations;
- UPS/BESS SOC, reserve, throughput, efficiency, losses, cycle/degradation and restart readiness;
- generator availability, start time, fuel and critical-load continuity.

### Grid
- interregional interface headroom, locational deliverability and N-1 sensitivity;
- LOLE event-periods, elapsed loss-of-load hours, LOLD event-days, EUE and declared energy-CVaR;
- ELCC/EFC with lower/upper bounds and exact adequacy reference metric;
- validated voltage/frequency deviation, ROCOF, settling, oscillation and power-quality metrics.

### Economics and environment
- total cost and marginal cost per delivered MW-hour and net MWh shifted;
- foregone or delayed compute value, queue/SLA penalties and revenue loss;
- equivalent BESS, thermal storage, onsite generation and network-upgrade alternatives;
- energy, emissions and water per completed useful-work unit;
- explicit hourly/site-specific emissions factor and lifecycle boundary.

### Uncertainty and generalization
- power-model prediction error and prediction-interval coverage;
- calibration, CT/PT, timestamp and paired-covariance uncertainty;
- out-of-domain abstention, false-pass and missing-data rates;
- expected and tail risk under correlated workload/weather/outage scenarios;
- common-mode dispatch failure, communication loss, repeated-event and recovery performance.

## Economics comparator: cost per reliable net delivered service

Use the companion `benchmarks/ai-data-centre-energy/v2/economics-known-truth.json` fixture and `ECONOMICS_COMPARATOR_PROTOCOL.md` for a frozen v0.1 calculation. Do not compare compute curtailment, a battery and thermal storage by nominal MW or installed $/kW alone.

An alternative is economically comparable only if it passes the same product contract: response latency, ramp, duration, sustained delivery, lower-bound net PCC energy after rebound, minimum availability and local safety/plant reserve. The denominator is expected net delivered MWh using a conservative availability lower bound and measured per-call net energy lower bound. Compute opportunity cost must be separately visible from control/software and recovery cost. Battery cost must include annualized capex, O&M, throughput degradation and recharge energy; thermal cost must include capital/O&M, water, cooling parasitics and recovery.

Publish cost per reliably delivered MWh and the break-even compute opportunity cost per call against the lowest-cost alternative that passes the same contract. If no alternative passes, report no comparable winner. All currency, real-price year, discount rate, lifetime, residual value and tax/subsidy treatment are caller-pinned. The IEA's compute-rental-versus-battery comparison is an industry signal, not a universal price input for this model.
## 6. Acceptance ladder

All thresholds below are proposed research targets, not current achievements or universal standards.

**Level 0 — deterministic truth**
- exact arithmetic, unit, timebase, sign and energy conservation;
- no hidden resampling or nested meter summation;
- all negative controls fail closed.

**Level 1 — public trace reproduction**
- reproduce Nature Energy 25% / 3 h field envelope under comparable workload/hardware/QoS;
- reproduce Alibaba's trace result under its published classification/window assumptions;
- compare MLPerf only within identical hardware/model/precision/quality strata;
- reproduce the ACM field-study ordering for launch and data semantics.

**Level 2 — held-out generalization**
- match or exceed the best eligible same-stratum MLPerf efficiency result; stretch goal ≥5% improvement with quality and uncertainty unchanged;
- target ≥25% deferral on a matched trace only if deadlines, p99 latency, queue drainage, facility energy/work and rebound do not regress;
- target <8% H100 power-model MAPE only on the same held-out domain where the published 11.4% baseline applies, with calibrated prediction intervals.

**Level 3 — plant and dispatch**
- repeated grid calls with bounded response latency, ramp, sustain, recovery and service impact;
- no use of critical UPS reserve; cooling and thermal limits remain satisfied;
- qualified PCC metrology with zero false passes in lower-bound threshold controls.

**Level 4 — system value**
- positive locational benefit remains under multi-region transfer and outage cases;
- no increase in the selected reference adequacy risk at the accredited target;
- demonstrate the risk-adjusted cost frontier against BESS, thermal storage and network alternatives.

**Level 5 — independent field replication**
- exact-head manifests and evidence receipts;
- independent reproduction of metrology, service, dispatch and grid claims;
- utility/operator and relevant test-lab acceptance before any market or grid-code claim.

## 7. Hard negative controls

- TDP masquerades as measured PCC power.
- Random split leaks sessions or sites across train and test.
- Clock skew manufactures apparent response.
- PCC + UPS + rack meters are added as independent measurements.
- Calibration expiry, unsupported operating range or clipped samples pass as valid.
- Paired covariance is assumed favorable because a meter is shared.
- A nominal reduction passes when its uncertainty lower bound crosses the contracted threshold.
- Rebound exceeds the initial event and is hidden by event-window truncation.
- Deferred jobs miss deadlines or backlog never drains.
- Read-write migration receives read-only latency assumptions.
- Battery dispatch consumes ride-through reserve or degradation is omitted.
- Correlated sites are treated as independent.
- PRAS LOLE event-periods are labelled as hours or its energy-CVaR is compared under an unpinned tail convention.
- A copperplate region is promoted to a nodal or substation deliverability claim.
- Synthetic, RMS/EMT, HIL or lab evidence is promoted to field performance.

## 8. v2.0 implementation sequence

1. Freeze the matrix, result schema, dataset digests and split registry.
2. Extend the PRAS harness with native capacity-credit calls, repeated seeds, matched Monte Carlo and weighted weather-year mixtures.
3. Add one public accelerator-trace adapter with held-out site/hardware splits.
4. Add BESS and cooling comparators with state-of-charge, degradation and coupled thermal state.
5. Replay the public field envelopes only where source artifacts and licensing allow.
6. Publish a compute-opportunity-cost versus grid-value Pareto frontier.
7. Move to owner-authorized PCC/workload evidence and independent field review.

## 9. Claim ceiling

A benchmark pass establishes only the metrics, model boundary, data split, operating conditions and claim class represented by its receipt. It does not itself prove utility acceptance, grid-code compliance, market accreditation, transmission deliverability, field performance or universal SOTA superiority.

## 10. Dataset scope guardrail: high-resolution GPU traces

The Scientific Data 2026 workload dataset (32 H100/B200 8-GPU node sessions, 40 RTX 3060 sessions, more than 1.8 million samples) is useful for accelerator and node-scale power model evaluation. The dataset README reports 15-minute sessions with 20 ms node-scale and 100 ms single-machine sampling. It is not a facility-PCC measurement chain, a three-hour dispatch demonstration, a cooling/ambient dataset or evidence of cross-data-centre correlation. Use it for per-GPU/node power prediction, workload phase characterization and within-node synchrony only.

The source repository README identifies the data as CC BY-NC-ND 4.0, while the Figshare DOI landing page displays CC BY 4.0. The licence is therefore unresolved pending rights-holder clarification. This benchmark does not copy or transform source files into Symtropy. Reuse, derivative analysis or distribution is blocked pending licence review; CI fixtures remain synthetic and source metadata only.

## 11. Counterfactual baseline and M&V

PR #3772 qualifies the PCC meter chain, but it does not establish the no-dispatch counterfactual. Issue #3802 / GRID-BASELINE-MV-001 introduces a separate gate: no-dispatch calibration groups and evaluation groups must be disjoint; forecasts must be issued before target intervals; baseline residuals include no-dispatch meter uncertainty; lower-tail residuals are calibrated at the independent-group level using the worst interval residual per calibration block; and evaluation regimes without calibration coverage abstain. Holdout bias, normalized error, block lower-bound coverage and false-positive reduction rate are reported by operating regime.

An event response lower bound is formed from the baseline lower prediction bound minus the upper PCC measured-load bound. It is model-based impact estimation, not causal identification. Recovery horizon completeness must be explicitly declared; event interval bounds are not a joint event-horizon probability bound without cross-interval dependence evidence.

## 12. Portfolio joint-delivery under shared failure domains

PR #3819 / issue #3817 adds a separate portfolio event-panel evaluator. It is not another load covariance calculation: #3783 characterizes observed cross-site power dependence, whereas this slice evaluates whether the full enrolled asset set delivers the contracted lower-bound kW during observed stress events.

Every event must include exactly one outcome for every enrolled asset and exactly one observed state for every declared failure domain. Missing outcomes/domain evidence fail closed; explicit Unknown remains Unknown. Per-asset receipts preserve the upstream response receipt ID and source digest. Domain receipts preserve observed state, evidence ID and source digest. The evaluator does not re-measure PCC or recertify site response.

Repeated dispatch episodes that share an incident-group ID use the group's worst event margin for group pass/fail. The report keeps event pass fraction and incident-group pass fraction separate. It also reports worst incident-group margin, maximum event shortfall, the empirical upper-tail mean of incident-group maximum shortfalls, scenario-specific group pass fractions, and observed portfolio pass/fail conditional on each domain state.

Failure-domain memberships can overlap. V0.1 reports the largest single-domain offered-capacity share and per-domain member outcomes, but it does not sum overlapping capacities or report an HHI without an explicit allocation model. The results are descriptive incident-panel evidence, not future failure probabilities, causal attribution, nodal deliverability or ELCC.

### Portfolio resilience benchmark gates

- Complete event × enrolled-asset and event × failure-domain panel; missing evidence never becomes zero or presumed success.
- Independent stress-group count is explicit and distinct from event count or interval count.
- The same complete panel is used to compare marginal site success, any declared independence estimate and observed joint portfolio success.
- All event receipts preserve per-asset response and per-domain evidence references.
- Shared-domain stratification is descriptive and never interpreted as causality.
- Overlapping failure-domain memberships cannot be added into total portfolio capacity.
- Future confidence intervals and scenario generation require separate independent-group statistical and empirical-validation protocols.

## 12. Locational network deliverability (GRID-LOC-DELIV-001 #3826)

Add a separate network gate between the PCC trajectory and the system-adequacy result. A response at the site meter does not prove that the response relieves the binding interface or feeder. Report three separate quantities: (1) site-meter response; (2) response deliverable at the relevant bus/interface under the frozen topology and contingency; and (3) probabilistic adequacy contribution.

The first synthetic crate will freeze the network model, topology timestamp, PCC-to-bus mapping, branch/interface ratings, monitored constraints and contingency set. It will implement DC power-flow/PTDF screening and LODF-based contingency checks with hand-computed three-bus fixtures. It must fail closed on islands, singular topology, stale snapshots, missing injection mapping, incorrect sign and rating violations. The report must disclose binding constraints and topology.

DC screening is not AC power-flow validation. Voltage, reactive power, losses, distribution transformer/feeder constraints require an independent AC solver on identical inputs. Protection, RMS/EMT transient stability and HIL remain separate gates. An N-0 result is not an N-1 result. Network deliverability is not ELCC, and a PRAS 0.9 adequacy result is not a nodal AC certificate.

Benchmark known truths: analytic three-bus PTDF shifts; equal MW response at high- and low-sensitivity buses; relief of one constraint with overload of another; N-1 reversal; islanding; bad bus mapping; sign reversal; stale topology; DC screen pass/AC voltage failure; correlated portfolio sites and cross-product double-counting. Targets are 100% known-truth fixture agreement within declared numerical tolerance and 100% fail-closed on invalid topology/mapping.
### Implementation status update (6 November 2026)

PR #3827 (14 Rust tests authored; exact-head CI pending) opens the first Rust DC screening package: a frozen network model and topology, an explicit MVA base, slack-bus balancing, chronological non-slack injections, signed per-bus positive load reductions, branch flow deltas, normal/emergency ratings, maximum action-fraction intersections and explicit N-1 outage recomputation. The synthetic three-bus case demonstrates a 30 MW requested reduction at bus B that relieves the A-B branch by 10 MW but increases S-B absolute flow by 20 MW; with the configured 45 MW S-B limit, only 17.5 MW (58.33%) is network-feasible at N-0. The PCC bus is pinned in the identity and must resolve to a non-slack bus in the frozen topology. This is synthetic DC arithmetic, not a real-grid deliverability claim. CI is pending.
## 13. Large-load disturbance performance (GRID-DISTURBANCE-001 #3851)

Add a separate conformance gate for operator-defined voltage/frequency ride-through, RoCoF, connection continuity and post-fault P/Q recovery. A sudden facility trip during a grid disturbance is not dispatch flexibility; it can create a system disturbance comparable in consequence to a large generation loss. The conformance engine must use the exact operator requirement version and PCC measurement-chain envelope.

The first slice is trace-based and bounded: voltage/frequency/RoCoF time series, facility connection status, active/reactive power, current where measured, declared UPS/backup mode, pre-event power reference, staged recovery and repeated-event behavior. It must abstain if critical samples or the required measurement bandwidth are missing.

Keep dispatch and disturbance channels separate. A load reduction during an overlapping fault cannot be credited as intentional demand response without causal attribution. A trace-conformance pass is not positive-sequence dynamic model validation, EMT/PDT stability, protection coordination, harmonic compliance, grid-code certification or ELCC.
## 14. Large-load disturbance conformance (GRID-DISTURBANCE-001 #3851)

Add a separate, operator-specific conformance gate for voltage/frequency ride-through, RoCoF, PCC connection continuity and post-fault active-power recovery. The Rust trace evaluator checks the pinned requirement version against PCC RMS waveforms, timebase, clock uncertainty, measurement bandwidth and RMS window. It evaluates allowed momentary cessation duration and recovery dwell/deadline/ramp/overshoot.

A sudden facility trip during a grid disturbance is not demand response. The conformance report always sets intentional demand-response credit to false. Dispatch flexibility and disturbance performance remain separate evidence dimensions.

A trace-conformance pass is not converter internal stability, positive-sequence dynamic model validation, EMT/PDT, protection coordination, harmonic/unbalance compliance, grid-code certification, interconnection approval or ELCC.
### Additional v0.1 measurement gates

The trace contract now requires a Nyquist-consistent relationship between maximum sample gap and declared minimum observable bandwidth. The first waveform sample must also arrive within the permitted gap after event start. If the operator requirement includes a reactive-power range or PCC current ceiling, every sample must contain those channels; missing Q/current data abstains, and measured values outside the operator envelope fail conformance. These are observable trace checks, not converter-model validation.