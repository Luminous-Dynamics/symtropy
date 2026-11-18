# Water platform: 2026 SOTA gap and benchmark strategy v0.1

**Date:** 18 November 2026  
**Scope:** water-network leak diagnosis, safe information acquisition, WaterLoop service delivery and South African deployment evidence.  
**Evidence policy:** paper-reported results are not treated as reproductions; simulated benchmarks are not field performance; synthetic CI passes are software-contract evidence only.

## Executive thesis

Do not compete by training the largest leak classifier or claiming one accuracy number. The defensible product boundary is an **evidence-qualified water operations system**:

`sense -> establish observability -> detect -> bound severity -> localize or abstain -> choose a safe inspection/probe -> confirm in the field -> verify repair and delivered service`

That architecture targets the bottleneck visible across 2026 literature: detection scores are comparatively mature, but spatial precision, generalization across utilities, uncertainty, safe intervention and independently verified outcomes remain weak.

## 1. Current research reference points

| Work | Reported result | What it actually establishes | Limitation / comparison rule |
| --- | --- | --- | --- |
| Ranganathan et al., Journal of Water Process Engineering (May 2026), DOI 10.1016/j.jwpe.2026.110114 | 47,000+ EPANET/WNTR-generated Net3 scenarios; leak detection >95% accuracy | A broad synthetic architecture benchmark for ML, DL, GNN and Transformer families | Not a field transfer result; localization remains harder. Reproduce the exact split, task and scenario generator before comparing |
| Wang, Water Research 295 (1 May 2026), DOI 10.1016/j.watres.2026.125538 | 95.6% recall and mean BattLeDIM2020 economic score €422,660 ± €11,553 for concurrent abrupt/incipient leaks | Strong hybrid hydraulic residual + graph-transformer comparator for multi-event diagnosis | Simulated SCADA training; BattLeDIM benchmark validation; data available on request. Treat as a reproduction target, not independent field evidence |
| Sparse pressure-flow Moroccan district study, Aqua (2026), DOI 10.2166/aqua.2026.300 | 648 scenarios; PR-AUC 0.901 detection, 0.774 severity, 0.700 zone; nearest-sensor accuracy 0.423 (0.185 system-level); Top-5 pipe 0.122 over 171 candidates | Direct evidence that diagnostic resolution degrades sharply from event detection to pipe localization under sparse sensing | One calibrated district network and controlled scenario generation. The paper attributes the ceiling primarily to observability/coverage, not simply model class |
| Active Valve Probing, IEEE Access 14 (28 Aug 2026), DOI 10.1109/ACCESS.2026.3728041 | L-Town top-1 localization: 30.0→96.7% at σ=0.002; 20.0→56.7% at σ=0.005; 6.7→60.0% at σ=0.010. Bench HIL: 32/36 vs 21/36 passive | Counterfactual/active hydraulic perturbations can create discriminative observations when passive sensing is ambiguous | Specific 20-candidate, six-valve study and small HIL sample. Reproduce the exact protocol and service constraints; do not generalize 96.7% as a utility-wide rate |
| Farah & Shahrour, Water 18(18) (11 Sep 2026), DOI 10.3390/w18182267 | Review of 90 studies; 30 classified public-benchmark, but only 23 used the three named water-leak resources tracked | Benchmark fragmentation remains material | Public topology reuse is not the same as independent SCADA event validation |
| Ruiz-Diaz et al., Water 18(21) (9 Nov 2026), DOI 10.3390/w18212797 | Review of 91 publications (70 primary, 21 reviews); only 37/70 primary studies used hydraulic sensing | Multimodal sensing, hybrid processing and multi-task diagnosis are still open | A taxonomy/review, not a direct leaderboard |
| Tarawneh, Water Practice & Technology (30 Oct 2026), DOI 10.2166/wpt.2026.485 | Review + benchmark of 244 records; 75% of pure-DL studies used simulation; only 11.3% reported inference time | Deployment reporting is inadequate; compute and reproducibility matter | Reproduce task/split and distinguish the paper's own benchmark from cross-study review counts |

### Main lesson

A >95% synthetic detection score is no longer an adequate differentiator. The Moroccan results show a detector with PR-AUC 0.901 can still have only 0.122 Top-5 pipe accuracy. The active-probing paper shows the observation process itself can change identifiability. Therefore optimize *diagnostic resolution and verified intervention value under safety constraints*, not classifier accuracy in isolation.

## 2. Paradigm shifts worth pursuing

### A. Observability before model capacity

Build a topology- and failure-domain-aware answer to: which critical states remain directly observed, under which sensor failures, calibration drift, clocks, communications, gateways and topology revisions? Our T0 executor (#5047 / PR #5889) and order-0/1/2 survival frontier (#5893 / PR #5898) are the first software-contract layer. Next improvement: compare passive sensing layouts using worst-case cut sets and service-critical state weights supplied by a utility, not a generic sensor count.

### B. Passive + active sensing co-design

Combine permanent sensors, temporary loggers, manual inspection and only then simulated valve probes. Use bounded response intervals and worst-case ambiguity rather than invented priors. A probe is useful only when it separates hypotheses under uncertainty and passes every hydraulic and operational precondition. The new active-observability T0 in #6015 formalizes this without an actuator path.

### C. Diagnosis becomes a decision with an abstention contract

Separate T1 event detection, T2 zone/pipe localization, T3 severity/flow estimation and T4 inspection/repair confirmation. Each task can abstain at its evidence ceiling. Report coverage and selective risk together; do not count unlocalized alerts as pipe-level successes.

### D. Optimize verified intervention outcomes

Move the headline metric from `accuracy` to:
- confirmed leaks per crew-day;
- litres/day recovered after independently verified repair;
- cost and elapsed time per confirmed leak;
- false excavation / unnecessary valve operation rate;
- service interruption and pressure-floor violations;
- uncertainty and evidence completeness at each handoff.

These metrics connect models to the utility's economic and water-loss outcome. They require field evidence and cannot be fabricated from simulation.

### E. WaterLoop: credit the useful service, not the treated volume

Treat gross product, quality-released product, fixture delivery, actual use, displaced mains volume, residuals and energy as separate ledgers. The matched service protocol (#5908 / PR #5909) and paired evaluator (#5911 / PR #5941) follow this principle. A water litre earns credit only at an exact service boundary and use profile, with a matched control and uncertainty screen.

### F. Two coupled twins, not one self-certifying digital twin

Keep a hydraulic model for conservation/physics and an independent evidence twin for sensor validity, model residuals, configuration, permissions, uncertainty, provenance and claim eligibility. Disagreement between the twins should widen uncertainty or force abstention, not cause one to silently overwrite the other.

## 3. Recommended leak benchmark suite

| Tier | Required metrics | Comparison design |
| --- | --- | --- |
| T1 detection | Event recall, precision, PR-AUC, false alarms per DMA-week, detection delay, missed-leak volume | Fixed event definition and false-alarm budget; include demand changes, pump/valve events, bursts, incipient leaks and concurrent leaks |
| T2 localization | Zone Top-1/Top-5; pipe Top-1/Top-5 only with pipe-level ground truth; P50/P95 graph distance; candidate-set size; abstention coverage | Same sensor budget, topology, demand, noise, label granularity and event holdout; report conditional accuracy and end-to-end coverage separately |
| T3 severity | Leak-flow MAE/RMSE, interval coverage, interval width, calibration and size-stratified error | Independent flow ground truth; held-out leak sizes and operating regimes; no use of simulator labels in test-time calibration |
| T4 intervention | Crew-minutes to confirmation, confirmed leaks per crew-day, false excavations, cost per confirmed leak, time-to-repair, verified post-repair loss reduction | Prospective utility-approved trials; blinded ranking where practicable; independent confirmation of leak and repair |
| Robustness | Performance under sensor dropout, shared power/gateway/clock/calibration failures, topology drift, missing data, out-of-distribution conditions | Cross-network and cross-season splits; leave-one-network-out; exact failure-domain cut sets |
| Deployment | Edge latency, memory, bandwidth, energy, model update/retraining time, operator override/abstention | Report hardware and communication conditions; benchmark on edge devices rather than only cloud/offline inference |

### Proposed stretch targets (research targets, not achieved results)

- Reproduce the Net3 >95% detection benchmark and Wang's BattLeDIM comparator under their exact published protocol before claiming any improvement.
- At matched event recall, aim for 25% fewer false alarms per DMA-week than the strongest reproduced baseline.
- At equal sensor budget, aim for at least 25% lower P95 localization distance or 25% lower worst-case candidate-set size; report both and abstention coverage.
- For the active-probe benchmark, first reproduce all three published L-Town noise levels and HIL sample counts. Only then target at least +10 percentage points Top-1 at the highest tested noise without a service-floor or safety-gate violation.
- Target zero false-feasible safety-gate cases in the synthetic active-observability suite; physical safety requires independent validation and is not inferred from this software metric.
- In any field pilot, require independently confirmed leaks and post-repair water-loss measurements before claiming litres saved.

## 4. WaterLoop and household reuse comparison

| Reference | Published/certified boundary | Correct comparison |
| --- | --- | --- |
| JoJo municipal-backup tanks | Storage capacity and pump/automatic-fill/pressure-switch features vary by model | Compare usable reserve at declared outlet pressure, delivered litres, continuity, pump energy, installation and maintenance; nominal tank volume is not service volume |
| Hydraloop H300/H350 listing | NSF/ANSI 350 Class R, Laundry & Bathing, rated 52.8 US gal/day (~200 L/day) in NSF's listing current 17 Nov 2026 | Compare only within the same classified end-use scope and actual measured delivery boundary; manufacturer's potential throughput is not the same as certified rated capacity |
| Moen Flo-type monitoring | Flow/pressure monitoring, usage-pattern alerts and automatic shutoff are established product capabilities | Differentiate with observability qualification, evidence-backed localization, safe abstention, independent repair confirmation and false-shutoff/service impact—not a generic claim of leak monitoring |
| Luminous WaterLoop | Current assets: storage/service accounting, preliminary P&ID/sizing, matched trial plan and synthetic service-credit evaluator | Not yet a physical product or treatment claim. Compare only after matched physical trials, verified use-quality release, actual fixture use, mains displacement, energy and residual handling |

NSF's official listing states Hydraloop H300/H350 at 52.8 gal/day, Class R, Laundry & Bathing; H600 at 260 gal/day in the same class. This is a certified scope boundary, not proof of suitability for potable use or every non-potable end use.

## 5. South African deployment constraints

The revised Compulsory National Water and Sanitation Services Standards were gazetted on 6 June 2025 (Government Gazette 52814, GN 6292). They define drinking water against SANS 241. Regulation 12 requires a Water Services Institution planning to separate, collect and use greywater beyond a household boundary to identify and manage the risks in the wastewater risk abatement plan, with the 2018 WRC greywater guidance used to support identification. The system must therefore separate source, treatment, sample, intended end use, release and post-treatment monitoring; a pump running or a clean-looking outlet is not a water-quality clearance.

## 6. Numerical engines and industry boundary

Do not rebuild EPANET or WNTR. EPA EPANET already solves network hydraulic heads and link flows, supports pumps/valves, multiple demand patterns, pressure-dependent demand and extended-period analysis. WNTR wraps EPANET simulation and offers a second simulator. Luminous should own the evidence and decision architecture around these engines:

- topology/version and data-provenance qualification;
- scenario and train/test split reproducibility;
- physics residual and cross-model discrepancy analysis;
- failure-domain observability and sensor/probe co-design;
- uncertainty, OOD detection, abstention and operator review;
- action/repair evidence and outcome verification;
- service, energy, lifecycle and environmental accounting.

## 7. Immediate execution sequence

1. Reproduce the 2026 Net3, BattLeDIM and sparse-Moroccan references as separate protocols; never pool their scores.
2. Add the new active-observability T0 (14 cases) to CI; use it to test set-valued ambiguity and no-probe abstention.
3. Feed the observability survival vector and safe probe options into #4006 as additional Pareto dimensions; do not create a duplicate optimizer.
4. Extend #5018 with a controller proposal/abstention/fallback benchmark. It must consume independent observability dispositions and never self-certify them.
5. Continue #5941 matched-service evaluator and keep energy visible even for rejected/zero-credit blocks.
6. Secure permissioned utility data and a competent utility partner before any field leak-diagnosis or valve-probing claim.
7. Run the WaterLoop clean-water metrology review before any assembly/energization; physical trial remains unauthorized until the competent review is recorded.

## 8. Source register

- Ranganathan et al. (May 2026), Journal of Water Process Engineering 87, 110114. DOI: https://doi.org/10.1016/j.jwpe.2026.110114
- Wang (1 May 2026), Water Research 295, 125538. DOI: https://doi.org/10.1016/j.watres.2026.125538
- Sparse pressure-flow Moroccan district study (2026), Aqua. DOI: https://doi.org/10.2166/aqua.2026.300
- Active Valve Probing (28 Aug 2026), IEEE Access 14, 134625–134635. DOI: https://doi.org/10.1109/ACCESS.2026.3728041
- Farah & Shahrour (11 Sep 2026), Water 18(18), 2267. DOI: https://doi.org/10.3390/w18182267
- Ruiz-Diaz et al. (9 Nov 2026), Water 18(21), 2797. DOI: https://doi.org/10.3390/w18212797
- Tarawneh (30 Oct 2026), Water Practice & Technology. DOI: https://doi.org/10.2166/wpt.2026.485
- NSF Official Listings, NSF/ANSI 350 Hydraloop entries (current 17 Nov 2026): https://info.nsf.org/Certified/Wastewater/Listings.asp?Company=C0427542&Standard=350
- South Africa, Government Gazette 52814, GN 6292 (6 June 2025): https://www.gov.za/documents/notices/water-services-act-compulsory-national-water-and-sanitation-services-standards
- US EPA EPANET: https://www.epa.gov/water-research/epanet
- US EPA WNTR hydraulics documentation: https://usepa.github.io/WNTR/hydraulics.html

**Evidence ceiling:** this document is a research and benchmark plan. The numbers above are published references, not Luminous results. The stretch targets are hypotheses. All current active-observability cases are synthetic; physical qualification, field performance and regulatory compliance remain NotRun.