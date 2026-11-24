# Mobility Response Settlement Evidence Bridge and Benchmark v0.1

**Programme:** MOBILITY-RESPONSE-SETTLEMENT-BRIDGE-001 (#7316)
**Source components:** #3649 dispatch response, #3772 PCC metrology, #3807 counterfactual baseline/rebound, #7313 mobility/network adapter
**Research cutoff:** 24 November 2026
**Implementation status:** Rust JSON report reconciler, strict schema, six-observation synthetic event/recovery fixture, CLI, package integration test and dedicated workflow authored; exact-head verification pending.

## 1. Why this layer matters

A site meter cannot directly answer what a facility would have consumed without dispatch. A baseline forecast cannot tell us whether a requested instruction was followed quickly enough. A response tracker cannot by itself establish that the PCC meter's measurement uncertainty is acceptable. A good PCC measurement does not establish that the response was network-deliverable at the constrained interface, and none of these establishes that a resource can restore mobility reserve before its next service obligation.

Treating any one of these reports as the final settlement quantity creates false credit: a high requested MW value may be only an offer; a fast response can be brief; gross reduction can be offset by rebound; a biased counterfactual can exaggerate savings; a meter uncertainty interval can include zero; and an N-1 network constraint can cap the deliverable quantity below what the site measured.

The new bridge composes the four existing report owners rather than duplicating their calculations. It is a cross-report evidence reconciler, not a meter, baseline forecaster, controller, optimizer, power-flow solver or settlement engine.

## 2. Paradigm shift: preserve four different response quantities

1. **Requested:** the quantity and time profile in the offer/instruction.
2. **Instructed and tracked:** the actual product instruction, response latency, sustained tracking, underdelivery, overshoot, ramp and signed import reduction.
3. **Measured at the PCC:** corrected power and energy with meter-chain identity, calibration covariance, clock alignment and expanded uncertainty.
4. **Counterfactual net impact:** qualified no-dispatch baseline minus actual dispatched power over the entire event and recovery horizon, including rebound.

Locational network deliverability is a separate constraint on the amount that can be used at the grid interface. Reserve recovery is another independent constraint. None of these quantities can be substituted for another or combined by taking a favourable maximum.

### 2.1 Evidence must compose across time and identity

The bridge binds event, offer, commitment epoch, service product, site, PCC, PCC bus, storage asset/configuration, meter-chain configuration, baseline model, response configuration, network model and topology. It verifies that instruction, event, dispatch, PCC intervals, recovery intervals and network offer use the same physical time boundaries.

Dispatch samples must align one-to-one with PCC metrology intervals and the corresponding baseline event evidence IDs. The baseline event includes all recovery intervals; missing recovery is not treated as zero rebound. Source reports with future, stale, missing or conflicting identities cannot be reconciled.

### 2.2 Technical reconciliation is not settlement authority

The output is a conservative technical evidence summary only. The bridge does not authenticate source signatures or GitHub provenance, identify a causal treatment effect, calculate a market settlement statement, determine a tariff, issue an invoice, authorise a payment, certify a market product, approve a connection or dispatch equipment.

All these flags stay false: source signatures verified, settlement authority, payment authorised, market eligibility, dispatch authority, interconnection approval and field qualification.

## 3. Existing source components and ownership

| Source | Owner responsibility | Report information consumed |
|---|---|---|
| #3649 — `symtropy-energy-systems::grid_response` | Product instruction and time-resolved tracking | Requested/delivered signed energy, latency, tracking, underdelivery, overshoot, ramp and sample evidence IDs |
| #3772 — `symtropy-pcc-metrology` | Calibration-aware metrology | Corrected PCC power, covariance-aware uncertainty, time alignment, signed interval energy, rebound and lower bounds |
| #3807 — `symtropy-counterfactual-baseline` | No-dispatch baseline qualification and event/recovery M&V | Regime qualification, event impact, gross reduction, rebound, conservative net response and recovery reconciliation |
| #7313 — `symtropy-mobility-network-adapter` | Mobility reserve/commitment evidence and locational DC screening | Exact offer/epoch/site/PCC/asset/bus/topology, upstream gates, requested/deliverable energy and N-1 result |

The bridge pins each report to its expected source PR, source code path and declared 40-character commit SHA. It computes a canonical digest over each embedded report. That is a report integrity check within this bundle, not proof that a source generated the report or authenticated GitHub provenance. Cryptographic producer/source authentication remains a Mycelix/trust-fabric responsibility.

## 4. Implemented v0.1 contract

Input is a frozen bundle of four serialized reports plus the cross-system identity and event time boundary. The bridge requires:
- an exact event, service product, instruction time and response-configuration digest;
- a matching site/PCC and meter-chain configuration;
- a matching baseline model and disjoint source partition report from the baseline owner;
- a matching offer ID and commitment epoch, asset/configuration, PCC bus, network-model digest and topology digest;
- exact grid instruction/service end and full counterfactual recovery horizon;
- identical PCC and counterfactual interval start/duration, contiguous across the event and recovery;
- exact grid response sample timestamps and evidence IDs matched to PCC and baseline reports;
- exact five upstream gates from the mobility/network report, with no duplicate or missing gate kind;
- all required component reports and their declared pass flags.

Source values remain separate. The bridge checks:
- PCC point net and summed interval lower-bound totals against the PCC interval receipts;
- PCC gross reduction minus gross rebound equals PCC signed net energy;
- baseline gross reduction minus rebound equals baseline net energy;
- baseline conservative net equals the sum of baseline interval lower-bound contributions;
- grid target kW × service duration equals requested response kWh;
- the grid requested quantity equals the mobility/network offer quantity;
- signed response tracking versus PCC dispatch and PCC net versus counterfactual net remain separately visible;
- the uncapped conservative technical lower bound does not exceed the DC-network-deliverable cap. A contradiction rejects reconciliation; it is never clipped into plausible credit.

The conservative event-plus-recovery energy lower bound is the minimum of the PCC summed interval lower bound and the counterfactual conservative net energy, subject to all source gates and the network cap. A separate **minimum dispatch-period service-power lower bound (kW)** is the minimum across dispatch intervals of the PCC calibrated lower bound, the counterfactual baseline lower bound and the matching per-interval DC network cap. The two values are intentionally not interconverted: the kW floor is never derived by dividing event-plus-recovery net kWh by dispatch duration. Neither value implies a joint probability across intervals. The baseline owner's `event_lower_bound_is_joint_probability` flag is preserved. The PCC and grid response flags for sub-interval transients remain explicit.

## 5. Frozen synthetic known answer

The fixture represents a 10 kW instruction lasting two seconds, followed by two recovery intervals. It is designed to demonstrate that event response and recovery are reconciled together.

| Quantity | Synthetic expected value |
|---|---:|
| Requested response energy | 0.0055556 kWh |
| Signed grid-response tracking energy | 0.0055556 kWh |
| PCC point net after rebound | 0.0054167 kWh |
| Counterfactual net after recovery | 0.0053056 kWh |
| Conservative technical lower bound, event + recovery | 0.0051944 kWh |
| Minimum dispatch-period service-power lower bound | 9.8 kW |
| Bound qualifies as a joint event probability | false |
| Source artifact signatures verified by bridge | false |
| Settlement/payment/market/dispatch/interconnection/field authority | all false |

These values are arithmetic/contract controls only. All four reports contain synthetic values. They do not demonstrate a physical response, an actual market settlement or a reliable service product.

## 6. Statistical and M&V boundaries

The counterfactual baseline remains an unobserved no-dispatch estimate during the event. A qualified out-of-sample residual lower bound is not causal identification. Randomised or credible matched-control event designs would offer stronger causal evidence; the v0.1 bridge does not perform those designs.

Summing interval lower bounds does not automatically create a joint event-horizon confidence bound. Dependence between intervals, common weather/price/asset failure, autocorrelation and data latency must be handled by the upstream source methods and separately qualified. The bridge preserves, rather than upgrades, those claims.

The dispatch response tracker measures at the declared sampling resolution; it cannot infer sub-sample transients. The PCC metrology report carries its clock alignment and expanded uncertainty assumptions; the bridge does not re-estimate them. The baseline event must include the full recovery window and signed rebound. A positive interval event response with negative net recovery is not silently converted into positive event credit.

## 7. SOTA and industry comparison

### 7.1 Joint-chance distributionally robust battery-swapping planning

Xin et al., *Distributionally robust location-inventory planning of commercial electric vehicle battery swapping stations with joint chance constraints*, Energy 355 (15 July 2026), 141090, formulate two-stage distributionally robust location/inventory planning with moment-based ambiguity sets and joint chance constraints. Their model uses a CVaR/SOCP reformulation and a tailored outer-approximation solver, with out-of-sample comparisons under heavy-tailed demand and a commercial DICOPT comparator.

Paper: https://doi.org/10.1016/j.energy.2026.141090

This should be a core planning comparator for fleet demand, inventory and service reliability. The mobility/network bridge adds evidence identity, exact PCC/topology binding, recovery and event metrology to the optimizer's candidate outputs; it does not claim a better optimizer than their SOCP/outer-approximation algorithm.

### 7.2 Distributionally robust reinforcement learning

Du and Zhu, *Optimal Scheduling of Electric Heavy-Truck Battery Swapping Stations via Distributionally Robust Deep Reinforcement Learning*, Applied Sciences 16(22), 11346 (November 2026), report DR-SAC with 78.9% peak shaving in their medium-demand peak window. Across the high-demand scenario, DR-SAC has 5.3 scarcity-violating periods/day, SAC 2.9, TD3 12.4 and DDPG 21.7; DDPG reaches 86.7% peak shaving but at 21.7 violations/day. The authors conclude that a safety layer is still needed under high demand. Their data represent one mining-area station and stated tariff/service definitions; these figures must not be transferred to an aerial mobility fleet.

Paper: https://doi.org/10.3390/app162211346

This is a strong argument for judging policies on a risk/quality/cost frontier, not reward or peak-shaving alone. Compare DR-SAC, SAC, TD3, MILP, MPC and robust optimization on identical mobility/charging/network scenarios, then pass all proposals through the same independent reserve, recovery, network and metrology gates. Report the opportunity cost of the shield and unsafe proposals rejected.

### 7.3 Cyber-physical resilient real-time operation

Ghelichpour et al., *Resilient real-time operational planning of smart grids with electric vehicle battery swapping stations*, Renewable Energy 275 (1 November 2026), 126219, incorporate communication delays, false-data injection attacks and network reconfiguration alongside multi-objective planning, fuzzy Monte Carlo uncertainty, loadability and dynamic security assessment on a modified 118-bus system.

Paper: https://doi.org/10.1016/j.renene.2026.126219

The benchmark extension is clear: event receipts, PCC baselines, meter clocks and network topology must be attacked and delayed as well as the physical storage asset. Test stale topology, measurement substitution, delayed instructions, false telemetry, replayed commitment receipts and communication loss.

### 7.4 Online temporal calibration and federated calibration

Huang, Ma and Michailidis, *Model-Agnostic Online Certificate-Driven Calibration for Time Series Forecasting Under Distribution Shift*, UAI 2026 / PMLR 337:2244–2273, use martingale PAC-Bayes certificates under temporal dependence and shift. Their predict-then-update protocol uses outcomes only after they become available and includes a gated residual Bayesian head over a fixed backbone. The v0.1 monitor from #7290 is a smaller auditable prequential baseline; it does not reproduce their PAC-Bayes guarantee.

Paper: https://proceedings.mlr.press/v337/huang26b.html

Shi et al., *Valid and Efficient Uncertainty Quantification for Federated Joint Shift*, UAI 2026 / PMLR 337:6232–6260, are the stronger federated calibration comparator. Reproduce FCPJS on its native benchmarks before claims about heterogeneous depot transfer. The fleet summary may not erase a local site's calibration alarm or abstention.

Paper: https://proceedings.mlr.press/v337/shi26a.html

### 7.5 South African service definitions and market context

NTCSA publishes the Ancillary Services Technical Requirements for 2026/27–2030/31, including reserve and system-restoration service sections, reactive-power/voltage control and constrained generation. Any mobility/ESS offer has to be mapped to an actual product's response, sustain, recovery, metering and locational requirements rather than called a generic firm MW resource.

Source: https://www.ntcsa.co.za/wp-content/uploads/2026/01/ASTR-2026_27-2030_31.pdf

The SAWEM Market Code Rev 2.3 (February 2026) defines Instantaneous Reserve as generating capacity or demand-side managed load able to respond fully within 10 seconds to a frequency drop, sustained for at least 10 minutes. That is a market-product definition, not a finding that this mobility offer qualifies or has participant rights.

Source: https://www.ntcsa.co.za/wp-content/uploads/2026/02/Market-Code-v2.3.pdf

On 7 October 2026, the South African Government announced a Section 34 determination prioritising 4,600 MW of BESS and 5,000 MW of gas-to-power capacity. The direction relates to new IPP procurement, not entitlement for a mobility fleet. It increases the relevance of a South African pilot, but connection, technical compliance, market participation and operator approval remain separate.

Source: https://www.gov.za/news/media-statements/minister-kgosientsho-ramokgopa-prioritises-battery-storage-and-gas-power

## 8. Benchmark tournament and target ladder

All comparators must receive the same event timeline, instruction, asset/configuration, site/PCC, meter chain, baseline partitions, weather/temperature, tariff, network topology, contingency set and recovery horizon. Hold out whole sites, hardware configurations and independent incident groups; do not random-split time-adjacent samples.

| Dimension | Benchmark / target |
|---|---|
| Identity/time | 100% exact agreement across event, offer, commitment epoch, asset, meter/PCC, product and timestamps |
| Interval evidence | 0 accepted sample-ID reuse, interval gap, boundary mismatch or missing recovery interval |
| Energy arithmetic | 100% reconciliation of source totals against signed interval contributions within frozen tolerances |
| Rebound | 0 positive credit when the qualified event-plus-recovery conservative net response is non-positive |
| Counterfactual baseline | 0 event qualification when baseline partition/regime support or full recovery evidence is absent |
| Network cap | 0 positive lower-bound credit when source lower bound contradicts maximum DC-network deliverability |
| Authority | 0 technical reconciliation promoted into market, settlement, payment, interconnection or dispatch authority |
| Determinism | 100% stable report-content digest for byte-equivalent report bundles |
| Runtime | Report p50/p95/p99 for 1k/10k/100k intervals, separate from upstream report generation |
| Statistical validity | False-qualification/abstention rates on held-out event and independent incident blocks, with cluster-aware uncertainty |

### 8.1 How to define a genuine SOTA win

A meaningful win is a Pareto improvement under identical constraints: more reliably delivered net kWh, better service completion and response latency, lower cost per reliable net delivered kWh, no increase in false-safe event qualification, and no loss of coverage on the worst-depot/worst-asset regime. The evaluation must include the cost of conservatism (abstained or curtailed events), while keeping hard-gate violations at zero. No single scalar reward can compensate for unsafe dispatch, an unqualified baseline or a market-rule failure.

### 8.2 Adversarial campaign

- Counterfactual baseline error depends on temperature, payload, mission arrival and weekday regime.
- Delayed or missing outcomes arrive after newer instruction events.
- Recovery energy rises after repeated shocks and correlated depot-wide events.
- Meter clock drift differs between the baseline and candidate channels.
- Gain/offset covariance is non-zero and becomes invalid after meter replacement.
- Gross dispatch reduction passes, but rebound erases the net event result.
- Grid response sample interval is too coarse for the product's response deadline.
- Network topology changes, line outage islands the PCC, or branch ratings are derated.
- The same site response is promised to multiple products or commitment epochs.
- One source report is regenerated at a new commit while the event bundle retains stale provenance.
- Fleet-level calibration looks acceptable but one local pack/temperature group is unsupported.
- False-data injection alters a single sample while aggregate average error remains low.

## 9. Next increments

1. Complete exact-head format, package tests and Clippy for the report bridge.
2. Publish a report-bundle artifact from the source PR workflows so the bridge can consume their actual versioned outputs rather than fabricated JSON.
3. Add cryptographic source receipt authentication through the correct custody/trust owner; never infer it from a digest string.
4. Add randomized/crossover or matched-control causal evaluation for event impact where operators can permit it.
5. Reproduce OMPB and FCPJS native benchmarks and compare on chronological, site-held-out mobility data.
6. Reproduce the 2026 two-stage joint-chance DRO and DR-SAC baselines on matched mobility/network test cases.
7. Add independent AC power-flow evaluation after the DC screen, then separate feeder, RMS/EMT/HIL and protection claim classes.
8. Run a first measured stationary-ESS/PCC depot campaign, including pre-dispatch baseline, instruction timestamps, high-resolution meter records and a complete recovery horizon.

## 10. Claim ceiling

The bridge is a read-only technical report reconciler. It does not identify causal impact, certify market eligibility, calculate a settlement invoice, authorise payment, compute ELCC, establish AC feasibility, qualify field hardware or issue dispatch/interconnection permission. Any real market/field claim requires independently authenticated source receipts, product-specific requirements and operator/market approvals.