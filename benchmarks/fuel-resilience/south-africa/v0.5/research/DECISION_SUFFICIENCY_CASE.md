# SAFRB decision-sufficiency and active-diagnosis case

**Case ID:** `SAFRB-VOI-001`  
**Research cut:** 24 November 2026  
**Status:** synthetic finite-world reference case; not a calibrated South African incident model  
**Consumer:** Symthaea active diagnosis / VoI / robust evidence-acquisition work  
**Related Symthaea issues:** #12403, #12515, #13365, #12173  
**Physical/service boundary:** SAFRB v0.2–v0.5

## Why this case exists

Fault classifiers normally receive a pre-selected observation set. This case asks a more demanding question:

> Given a possible fuel-to-service shortfall, should the system act now, request another measurement, choose a different test, or stop because further evidence cannot change the best safe decision?

The objective is decision quality, not entropy reduction. The best measurement depends on the action losses, test likelihoods, acquisition cost, delay, source dependence and the deadline for restoring service.

This case is a finite exact-enumeration oracle for the small decision model. It is not an estimator of real-world prevalence or a field-calibrated EVSI model.

## Competing hypotheses

| ID | Hypothesis | Nominal prior |
|---|---|---:|
| H1 | Import/cargo delay or physical supply unavailable | 30.00% |
| H2 | Product quality hold, grade mismatch or unusable batch | 25.00% |
| H3 | Generator underperformance / auxiliary-load increase | 25.00% |
| H4 | Demand estimate or service-meter artifact | 20.00% |

The nominal priors sum to 100%. They are deliberately synthetic. A future robust track should replace the point prior with an evidence-supported admissible prior set, not pretend this table represents national frequency.

## Immediate response actions and loss matrix

Loss values are abstract benchmark points. They are not currency and must not be interpreted as Rand. They combine service interruption, unnecessary procurement, unnecessary isolation, diagnostic delay and repair/reconfiguration consequences. Hard quality, safety and legal constraints remain non-tradeable constraints outside this scalar loss.

| Action | H1 import delay | H2 quality hold | H3 generator underperformance | H4 demand/meter artifact |
|---|---:|---:|---:|---:|
| A0 wait / monitor | 180 | 170 | 140 | 0 |
| A1 procure alternate supply | 35 | 160 | 145 | 80 |
| A2 quarantine / replace batch | 145 | 30 | 140 | 90 |
| A3 repair / reroute generator | 150 | 145 | 35 | 90 |
| A4 validate demand / meter | 150 | 150 | 130 | 20 |

With no additional measurement, the expected losses are:
- A0: 131.50 points
- A1: 102.75 points
- A2: 104.00 points
- A3: 108.00 points
- A4: 119.00 points

**No-test decision:** A1, alternate supply, with expected loss 102.75 points.

## Candidate measurements

Likelihoods are reported in basis points. For each hypothesis, the two outcomes of a test sum to 10,000 bps.

| Test | Positive outcome | H1 | H2 | H3 | H4 | Cost units | Nominal latency |
|---|---|---:|---:|---:|---:|---:|---:|
| M1 vessel status | delayed | 9,000 | 800 | 1,200 | 1,500 | 10 | 1 h |
| M2 quality sample | off-spec | 800 | 9,500 | 800 | 400 | 20 | 4 h |
| M3 generator load test | underperforming | 500 | 800 | 9,500 | 500 | 25 | 3 h |
| M4 independent demand check | discrepancy | 500 | 400 | 500 | 9,000 | 8 | 1 h |

The negative outcome is the complement for each hypothesis. These likelihoods are synthetic exact fixture values.

### Source-dependence fixtures

A second test must not automatically be treated as independent simply because it arrives through a different API or document:
- supplier EDI confirmation and a vessel-status portal may share a supplier / shipping source;
- dashboard tank level and a SCADA tag may share the same physical gauge, calibration epoch, historian collector or clock;
- an independent tank gauge may still share the same strapping table or temperature correction;
- a lab report may be authentic yet bound to a different batch or sample chain;
- generator controller telemetry and derived output can share one internal sensor/model.

The robust evidence-acquisition tournament must include these common-cause relationships and preserve their dependency identity. Reuse Mycelix's existing multi-valued dependency work (#5762), and Symthaea's dependence/model-family work (#10410, #12515). Do not multiply likelihoods as independent without evidence.

## Exact one-step decision-value oracle

For each measurement:
1. calculate each possible outcome probability from the nominal prior;
2. update the hypothesis distribution using Bayes' rule;
3. choose the minimum expected-loss action for each outcome;
4. average the post-measurement minimum expected loss;
5. subtract the measurement acquisition cost from the gross value.

`Net EVSI = Expected loss before test - Expected loss after test - Test cost`

Latency is reported separately and should enter an operational model as a time-dependent consequence, not be silently converted into the same units.

| Test | Expected loss after result (before test cost) | Gross EVSI | Test cost | Net EVSI | Outcome-dependent action |
|---|---:|---:|---:|---:|---|
| M1 vessel status | 76.750 | 26.000 | 10 | **+16.000** | delayed → A1; on-time → A2 |
| M2 quality sample | 74.495 | 28.255 | 20 | **+8.255** | off-spec → A2; normal → A1 |
| M3 generator load test | 78.150 | 24.600 | 25 | **−0.400** | underperforming → A3; normal → A1 |
| M4 demand check | 89.725 | 13.025 | 8 | **+5.025** | discrepancy → A0; no discrepancy → A2 |

Under this exact synthetic model, M1 has the highest net one-step value. M2 has the largest gross value before acquisition cost. M3 reduces expected decision loss but does not repay its stated acquisition cost. M4 changes the recommended action in both outcomes, yet has less net value than M1. This illustrates why "does it change the decision?" and "is it worth buying the information?" are related but distinct questions.

The fixture does not include deadline penalties, correlated tests, model ambiguity, posterior calibration error or measurement invalidity. Those belong in the next tournament layer.

## Required benchmark policies

1. Act immediately using current expected loss.
2. Fixed test schedule.
3. Random test selection under the same cost/time budget.
4. Maximum entropy / expected information gain.
5. Greedy one-step net EVSI.
6. Sequential adaptive EVSI with stop/continue decisions.
7. Robust/minimax EVSI over declared prior and likelihood intervals.
8. Source-dependence-aware evidence acquisition.
9. Human operator response with exactly the same evidence and test budget.
10. Exact adaptive decision-tree oracle for small cases only.

The oracle is an upper bound for this finite case. The production heuristic must not be called optimal unless the full admissible tree has been enumerated.

## Decision-sufficiency stopping rule

Stop acquiring evidence and choose an action when:
- all remaining admissible hypotheses/models support the same permitted action; or
- no available test has positive net decision value before the operational deadline; or
- the decision is already robustly acceptable under the hard service/quality/safety constraints.

Abstain or escalate when:
- plausible models recommend different high-consequence actions;
- required evidence sources are dependent or compromised;
- a required quality/calibration/authority predicate is unresolved;
- test likelihoods or loss values are not supported well enough to rank actions.

A low-entropy posterior is not sufficient if the remaining uncertainty can still change the decision.

## Metrics

- expected and worst-case decision regret;
- net EVSI and realized value of each measurement;
- shortfall volume and critical-service energy avoided;
- wrong-action and false-isolation rate;
- unnecessary test count and total test cost;
- time from initial signal to qualified action;
- measurement invalidity / common-source false corroboration;
- stop-too-early and continue-too-long rates;
- calibration of outcome likelihoods and posterior intervals;
- abstention quality and decision-sufficiency coverage.

## Claim ceiling

All numbers in this case are synthetic mathematical fixtures. The nominal priors, likelihoods, loss points, test costs and delays must be replaced or calibrated from independently reviewed operator evidence before this becomes a real decision-support evaluation. The exact oracle validates the arithmetic conditional on the fixture; it does not validate the assumptions or authorise a physical action.
