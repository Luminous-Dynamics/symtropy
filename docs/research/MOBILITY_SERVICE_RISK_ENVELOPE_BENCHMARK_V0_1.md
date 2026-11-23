# Mobility Service Tail-Risk and Probability Ambiguity Benchmark v0.1

**Parent:** [SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001 #6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921)  
**Nonanticipative source:** [#7133 / PR #7181](https://github.com/Luminous-Dynamics/symtropy/pull/7181)  
**Probability evidence owner:** [MFB-SCENARIO-CALIBRATION-001 #6460](https://github.com/Luminous-Dynamics/symtropy/issues/6460)  
**Implementation issue:** [#7196](https://github.com/Luminous-Dynamics/symtropy/issues/7196)  
**Branch:** research/mobility-service-risk-envelope-v0-1  
**Research cut-off:** 24 November 2026  
**Status:** source-level evaluator, strict probability manifest, synthetic fixture, CLI, 18 Rust test definitions and workflow integration; exact-head Rust verification pending.

## 1. The gap: exact policy does not imply calibrated risk

The service benchmark now distinguishes a clairvoyant full-horizon assignment upper bound (#7129) from an exact nonanticipative policy over shared information sets (#7133). That separation is essential: an online dispatcher cannot see future requests, weather, permissions or equipment outages before they are revealed.

A further gap remains. Even an exact optimal policy under declared scenario weights can be fragile when those weights are synthetic, uncalibrated, sparse, stale or misspecified. A high expected service score can hide severe failures in a small tail of the scenario distribution.

This child re-runs the upstream oracle and consumes the exact leaf outcomes and probabilities it emits. It does not accept manually authored leaf performance values or probability weights.

## 2. Paradigm shift: publish three distinct risk views

For the same frozen policy and joint leaf outcomes, publish:

1. **Nominal expectation:** exact expectation under the supplied finite joint-path probability vector.
2. **Tail outcomes:** fractional-mass CVaR for worst-tail on-time/completed service and worst-tail lateness/unserved demand.
3. **Probability ambiguity:** worst expected service and worst expected lateness inside an explicitly declared total-variation (TV) ball around the supplied probability vector.

The TV ball is a specific finite probability-mass perturbation set. It is not Wasserstein DRO, moment-based DRO or a general distribution-free guarantee.

Unweighted adversarial stress scenarios are a separate product. They must not be assigned convenient probabilities merely to enter expected-value or CVaR calculations.

## 3. Executable v0.1

The CLI receives four immutable inputs:
- the exact nonanticipative scenario tree;
- the linked service-quality input;
- the capacity architecture input that produces the mission opportunities;
- a probability-provenance manifest identifying the source and evidence class of the leaf probabilities.

The evaluator re-runs the nonanticipative service oracle before calculating risk. It derives the leaf IDs, probabilities and outcomes from that report, checks the leaf weights sum exactly to one, and emits SHA-256 digests of the canonical tree, linked service and capacity inputs.

The output contains:
- exact nominal expected on-time services, completed services and lateness;
- lower-tail CVaR for on-time and completed-by-horizon service;
- upper-tail CVaR for lateness and unserved-plus-rejected requests;
- TV-robust minimum expected on-time and completed service;
- TV-robust maximum expected lateness;
- worst-leaf service and lateness;
- exact per-leaf probability numerators and denominators;
- evidence-provenance basis and claim-ceiling flags.

All reported ratios use integer numerator/denominator pairs. The audit record is not a rounded floating-point score. CVaR consumes fractional probability mass at the tail boundary. The input parameter is tail mass, not confidence level: 100,000 ppm means the worst 10% of probability.

Run the evaluator:

    cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin mobility_service_risk_envelope_screen -- \
      tools/ev-passenger-cost-frontier/fixtures/nonanticipative_service_oracle_v0_1.json \
      tools/ev-passenger-cost-frontier/fixtures/nonanticipative_linked_service_v0_1.json \
      tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_capacity_v0_1.json \
      tools/ev-passenger-cost-frontier/fixtures/mobility_service_risk_envelope_v0_1.json

## 4. Probability evidence is a first-class input

The manifest distinguishes:
- synthetic;
- historical-frequency estimate;
- forecast-model probability;
- expert elicitation;
- stress-only;
- unknown.

Unknown and stress-only sets are rejected from probabilistic risk metrics.

The current v0.1 provenance rules require:
- historical frequency: source dataset and calibration-report digests, sample and effective sample sizes, and a held-out evaluation window strictly after the training cutoff;
- forecast model: source and calibration digests, training cutoff before forecast issue time, and evaluation beginning no earlier than the forecast issue;
- expert elicitation: an identified panel count and elicitation protocol reference;
- synthetic: permitted for mathematical fixtures only, with no calibration or empirical-risk claim.

The manifest contains geography, service population, service class, joint-event definition, data version, observation/evaluation windows, training cutoff, forecast issue time, sample count and effective sample size where applicable. It also carries the canonical SHA-256 digest of the exact scenario-tree contents; a digest mismatch is a hard rejection, even when the Git revision field is unchanged.

**#6460 remains the owner of probability calibration.** This module checks the provenance contract and chronology, but does not recompute Brier score, log loss, CRPS, calibration error, confidence intervals or empirical representativeness. A populated calibration-report digest is not itself proof that the calibration report passed.

The report includes a canonical digest of the actual supplied scenario-tree contents as well as hashes of the linked service and capacity inputs. The Git revision/reference in the probability manifest remains declarative provenance; it is not treated as a cryptographic signature.

## 5. Independent arithmetic and negative controls

The 18 focused Rust test definitions cover:
- a fractional lower-tail CVaR boundary;
- a fractional upper-tail CVaR boundary;
- a rare severe outcome that barely moves the nominal mean but strongly affects tail risk;
- TV radius zero reproducing nominal expectation;
- TV radius one moving all feasible mass to the adverse extreme, plus exhaustive three-leaf enumeration for every integer ambiguity radius;
- exact probability-mass normalization;
- rejection of stress-only and unknown probability bases;
- rejection of historical training/evaluation overlap;
- duplicate or missing leaf-manifest identities;
- scenario-order permutation and canonical digest stability;
- invalid tail mass and TV radius;
- end-to-end recomputation from the frozen upstream nonanticipative tree.

The independent three-leaf arithmetic case uses probability weights 50%, 30%, 20%. For a 40% lower-tail CVaR, the entire 20% worst leaf and only 20% of the middle leaf are consumed. For a 40% upper-tail lateness CVaR, the entire 20% most severe leaf and only 20% of the next leaf are consumed. This tests fractional mass rather than averaging a whole number of scenarios.

## 6. Relevant research and industry comparison

### Dynamic request admission under changing weather

Zhao, Zhang and Coelho, Transportation Research Part E 214 (October 2026), article 105074, formulate dynamic truck-drone request acceptance under changing wind as an MDP with approximate dynamic programming. Their method models requests arriving over time and uses wind-dependent energy and ground-speed feasibility. Their computational study includes instances approaching 300 requests. A secondary bibliographic record reports an 18.58% acceptance improvement against wind-unaware approaches on the paper's own cases.

This is a meaningful online policy comparator. Reproduce its native instances, assumptions and denominators before using its reported uplift as a target. The Luminous scenario-tree oracle provides exact labels on tiny nonanticipative instances; larger ADP/MPC comparisons must use the same observation-release schedule.

- [ScienceDirect abstract](https://www.sciencedirect.com/science/article/abs/pii/S1366554526004138)
- [DOI 10.1016/j.tre.2026.105074](https://doi.org/10.1016/j.tre.2026.105074)

### Distributionally robust battery-swapping policies

Du and Zhu, Applied Sciences 16(22), 11346 (16 November 2026), develop DR-SAC for heavy-truck battery-swapping station scheduling. Their robust MDP uses a KL-divergence ambiguity set and an offline VAE transition estimate. The paper reports 3.7% relative daily-profit variation under joint upward demand/price perturbation and a 78.9% peak-shaving result at a selected KL radius. It also discusses battery scarcity and remaining constraint violations under high demand.

This is a useful robust-learning comparator, but it is a road-EV station case with its own SOC discretization, data and objective. Compare it only after native reproduction, and place all policy proposals behind the same independent mobility reserve, recovery and commitment gates.

- [Open-access paper](https://www.mdpi.com/2076-3417/16/22/11346)
- [DOI 10.3390/app162211346](https://doi.org/10.3390/app162211346)

### Distributionally robust UAV fleet deployment

Xu, Li and Zhao, Transportation Research Part E 2026, article 105238 (available online 21 September 2026), formulate state-dependent demand uncertainty for UAV fleet/depot deployment with two-sided mean bounds and ambiguity sets. Their work provides a relevant network-planning comparator for the interaction between demand uncertainty and depot/aircraft capacity. It is not the same problem as the operational service-tree risk evaluator.

- [ScienceDirect abstract](https://www.sciencedirect.com/science/article/pii/S1366554526005764)
- [DOI 10.1016/j.tre.2026.105238](https://doi.org/10.1016/j.tre.2026.105238)

### Calibration is not an optimizer feature

#6460 requires rolling-origin chronological evaluation, training cutoff, held-out windows, source dataset/version, population, geography, effective sample size, and joint-path calibration. It also explicitly separates unweighted stress tests from probabilistic scenarios.

This division is correct:
- #7133 decides over information available at each decision node;
- #6460 qualifies the probability evidence;
- #7196 computes exact finite-scenario risk from those qualified inputs;
- #6803/#6809/#6829/#6837 remain hard feasibility, recovery, unique-promise and promise-persistence gates.

## 7. Fair tournament and how to beat SOTA

Use the same frozen workload, nonanticipative information schedule, event-derived capacity opportunities, probability manifest, compute budget and hard assurance gates for every policy.

Compare:
1. FCFS and EDF;
2. rolling-horizon MPC;
3. paper-native ADP and DR-SAC after source-native reproduction;
4. the exact nonanticipative oracle on tractable scenario trees;
5. clairvoyant full-information DP as a separate upper bound;
6. expectation-only, CVaR-aware and TV-robust policy selection;
7. unweighted adversarial stress suites separately.

Report the following side by side:
- nominal expected on-time service;
- lower-tail service CVaR;
- upper-tail lateness and unserved CVaR;
- TV-robust expected service under the declared radius;
- worst stress-case outcome;
- value of perfect information;
- online policy regret against the nonanticipative optimum;
- intervention/abstention rate of the hard gates;
- cost and energy premium for reduced tail risk;
- compute latency and whether the exact oracle completed or hit its bounds.

Never compare online ADP/MPC regret directly with the clairvoyant upper bound without reporting the value of future information separately. Never call a TV ball a Wasserstein or moment-based DRO guarantee. Never blend adversarial stress cases into the probabilistic CVaR denominator.

## 8. Target ladder (not achievements)

- 100% agreement with an independently implemented integer risk oracle on bounded leaf sets.
- 100% fractional-tail-mass correctness.
- TV radius 0 exactly reproduces nominal expectations.
- TV radius 1 reaches the adverse extreme for the declared finite outcome metric.
- Zero inclusion of unknown/stress-only cases in probabilistic metrics.
- Zero temporal leakage in historical/forecast provenance manifests.
- 100% leaf identity coverage and scenario-order-invariant risk results.
- Zero hard mission, reserve or permission violations admitted by risk aggregation.
- Target p99 below 10 ms for bounded 16-leaf cases on declared hardware.
- Publish risk/service/cost Pareto frontiers rather than a single composite score.

For larger tractable cases, target mean online-policy regret <=1% and p95 <=5% against the nonanticipative oracle. This is a research objective, not an achieved result, and remains separate from empirical probability calibration.

## 9. Phasing

- v0.1: exact finite-leaf expectation, CVaR and TV sensitivity; strict provenance manifest; synthetic fixture.
- v0.2: consume #6460 calibration receipts and validate chronological partitions/calibration diagnostics without reimplementing forecast generation.
- v0.3: separate unweighted stress-set evaluator and correlated compound-hazard families; stress cases remain outside probabilistic CVaR.
- v0.4: compare TV, Wasserstein and moment-based ambiguity sets with their own assumptions and evidence; do not relabel one as another.
- v0.5: held-out demand/weather calibration, regional drift, importance-weighted stress discovery and independent evaluation where source-data rights permit.

## 10. Claim boundary

A passing v0.1 run proves exact risk arithmetic over the supplied finite scenario tree and declarative probability manifest. Synthetic weights prove arithmetic only. It does not prove calibrated probabilities, that the TV radius is empirically justified, that the ambiguity set contains the true distribution, that the scenarios cover the field, or that any flight, market offer or dispatch is authorized.
