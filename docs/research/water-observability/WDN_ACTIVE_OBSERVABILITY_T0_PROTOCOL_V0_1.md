# WDN active observability T0 protocol v0.1

**Tracking issue:** #6015  
**Passive placement:** #4006  
**Observability qualification:** #5045/#5047  
**Survival frontier:** PR #5898  
**Date:** 18 November 2026  
**Evidence class:** synthetic-only counterfactual contract.

## 1. Research motivation

Passive leak detection is not enough when several fault hypotheses produce the same measurement pattern under sparse sensing. A 2026 IEEE Access study tested active valve probing with two pressure sensors and a deterministic response library. On its L-Town experiment with 20 candidate leaks and six probing valves, it reported localization accuracy rising from 30.0% to 96.7% at one tested noise level; the gain was smaller at higher noise. Its preliminary hardware-in-the-loop experiment reported 32/36 correct localizations compared with 21/36 under passive monitoring. Those are study-specific results and small test populations, not a general utility deployment guarantee.

The research shift is from passive sensing alone to **counterfactual active observability**: determine which observation would separate the remaining candidate hypotheses, and reject a proposed experiment unless its operating constraints are supported.

## 2. Why set-valued ambiguity instead of expected information gain

Expected information gain requires a defensible prior over hypotheses and a model of action outcomes. In sparse utility networks those priors are often unknown or badly calibrated. v0.1 therefore uses deterministic, unweighted response intervals.

For each candidate hypothesis and observation feature, record a closed response interval [lower, upper] that already includes the declared model and measurement uncertainty. Two hypotheses remain indistinguishable when their intervals overlap on every shared feature. This creates an undirected indistinguishability graph. The exact maximum clique size is the worst-case number of mutually indistinguishable hypotheses.

The evaluator enumerates all non-empty hypothesis subsets under an eight-hypothesis limit and reports all maximum cliques. No probability, likelihood, expected value or causal truth is inferred.

## 3. Passive and active observation model

Each case contains:
- a bounded set of candidate hypotheses;
- passive sensor response intervals for each hypothesis;
- one or more candidate observations;
- a minimum required reduction in worst-case ambiguity.

Probe types:
- `temporary_sensor`: a temporary, independently bound pressure/flow/acoustic/energy measurement;
- `operator_observation`: a manual meter, valve-state, acoustic or field observation;
- `simulated_valve_perturbation`: a counterfactual pressure-response observation only.

v0.1 evaluates every feasible single probe and every pair of feasible probes. Sequential probe features are concatenated as independent, namespaced observations. The output is a counterfactual recommendation only; the evaluator emits no valve command, setting, setpoint or actuation payload.

## 4. Safety/feasibility gates

Every probe must carry current, explicit evidence for:
- hydraulic model and topology revision;
- valid operating window;
- valid telemetry time and calibration;
- pressure-floor margin above the required minimum;
- fire-flow/emergency reserve above its required minimum;
- maximum transient below its declared limit;
- no water-quality hazard or isolation conflict;
- reversible rollback defined;
- operator authorization, permit approval and active lockout clearance.

A simulated valve probe additionally requires independently verified valve identity and current valve position. Missing, stale, contradictory or out-of-scope values make the probe infeasible. Thresholds are not relaxed to make a probe rankable.

These are software preconditions in a synthetic case. Passing them does not establish physical safety or authorize an operation.

## 5. Decision rule

1. Compute passive maximum-clique ambiguity.
2. Exclude any probe with one or more failed feasibility gates.
3. Compute post-observation maximum-clique ambiguity for all feasible singles and pairs.
4. Retain only sequences meeting the case's minimum worst-case reduction.
5. Rank deterministically by smaller post-probe maximum clique, fewer probes, shorter declared duration, then lexical probe IDs.
6. If no probe is feasible, abstain. If no feasible sequence meets the declared worst-case reduction, abstain.

Do not promote a probe on a mean-like score when it fails the declared worst-case ambiguity criterion.

## 6. Frozen 14-case challenge

1. Safe temporary sensor separates four passive-equivalent hypotheses.
2. Manual observation reduces four candidates to two.
3. Valve probe rejected by pressure-floor constraint.
4. Valve probe rejected by fire-flow reserve.
5. Valve probe rejected by transient limit.
6. Valve identity/position unverified.
7. Hydraulic model/topology stale.
8. Telemetry time/calibration invalid.
9. Safe probe adds no separation under uncertainty intervals.
10. Weak probe fails the declared minimum worst-case reduction.
11. Two individually insufficient probes jointly separate all candidates.
12. Hypothesis/observation input order permutation invariance.
13. Missing operator authorization.
14. No feasible probe; explicit abstention.

## 7. Benchmarks

Primary metrics:
- maximum-clique ambiguity before/after;
- worst-case ambiguity reduction;
- pairwise indistinguishability edge count;
- percentage of unsafe/infeasible probes correctly rejected;
- safety-gate false-feasible count (target: exactly zero);
- correct abstention when no feasible or sufficiently informative probe exists.

Secondary metrics for future physical studies:
- top-1/top-5 localization under matched candidate sets;
- time and crew-minutes to acquire a discriminative observation;
- temporary sensor budget and number of invasive interventions;
- pressure and fire-flow margin consumed;
- service interruptions, energy and water impact.

Compare passive sensors only, passive plus temporary sensors, passive plus manual observations, and passive plus simulated valve probing at identical candidate sets, topology, demand, uncertainty and sensor budget.

## 8. Claim ceiling

A passing T0 receipt establishes only deterministic interval-compatibility and fail-closed feasibility behavior on synthetic fixtures. It does not demonstrate live observability, hydraulic-model validity, field localization accuracy, valve safety, customer continuity, utility approval or deployment. Any physical probe requires independent hydraulic safety review and explicit utility authorization.