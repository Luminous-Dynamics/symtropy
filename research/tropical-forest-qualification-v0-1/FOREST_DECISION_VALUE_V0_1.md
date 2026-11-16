# Forest Decision-Value Tournament v0.1

**Tracking:** [FOREST-DECISION-VALUE-001 #5223](https://github.com/Luminous-Dynamics/symtropy/issues/5223)  
**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Reuse:** #2171 active sensing / EVSI, #2244 model-structure uncertainty, #2334 model-set adequacy  
**Status:** public synthetic development cases; no field action authorized

## 1. Paradigm: uncertainty is not decision value

A site can have high predictive variance and zero decision value when every plausible observation leaves the optimal action unchanged. A site with moderate uncertainty can have high decision value when one affordable measurement can switch the optimal action and reduce expected loss.

The target is not “sample the most uncertain pixel.” It is “sample the feasible site whose result is expected to improve the declared decision enough to justify cost, access, time and ecological disturbance.”

Use the existing Symthaea EVSI / active-sensing implementation. This benchmark owns forest decision contexts, candidate-measurement semantics, public synthetic expected cases and fair comparisons; it does not introduce a second generic information-theory kernel.

## 2. Worked synthetic example A: high uncertainty, zero decision value

A site has a binary latent state (high versus low species turnover) with prior probability 0.5. Both available actions have the same declared loss in both states:

| Action | High turnover | Low turnover |
|---|---:|---:|
| Protect | 20 | 20 |
| Do not protect | 0 | 0 |

A perfect measurement would eliminate all uncertainty about turnover, but it cannot change the preferred action because the decision loss does not depend on that state. Decision-focused EVSI is exactly zero before measurement cost. An entropy-only policy may select it; a decision-value policy should not.

This is a deliberate anti-objective example: information gain is not rewarded unless it improves a declared decision.

## 3. Worked synthetic example B: moderate uncertainty, positive decision value

A forest block has probability 0.4 of a consequential degradation state. Available actions are protect or do not protect:

| Action | Stable | Degraded |
|---|---:|---:|
| Protect | 30 | 30 |
| Do not protect | 0 | 100 |

Without another measurement:
- expected loss of Protect = 30;
- expected loss of Do not protect = 0.6×0 + 0.4×100 = 40;
- the current preferred action is Protect, at expected loss 30.

A synthetic field observation has sensitivity 0.8 and specificity 0.8:
- probability of a positive signal = 0.4×0.8 + 0.6×0.2 = 0.44;
- posterior degradation probability after positive signal = 0.32/0.44 ≈ 0.7273; Protect remains preferred, expected loss 30;
- probability of a negative signal = 0.56;
- posterior degradation probability after negative signal = 0.08/0.56 ≈ 0.1429; Do not protect is preferred, expected loss ≈14.2857.

Expected post-observation decision loss = 0.44×30 + 0.56×14.2857 = 21.2. Thus:
- gross EVSI = 30 − 21.2 = **8.8 synthetic loss units**;
- at measurement cost 3, net expected value = **5.8 synthetic loss units**.

This is not an estimated real conservation effect. It is a transparent known-truth arithmetic case showing why decision relevance matters.

## 4. Tournament baselines

At matched candidate-site set, budget, evidence, measurement likelihoods and constraints compare:
- random eligible-site selection;
- systematic grid / fixed rotation;
- maximum predictive variance reduction;
- maximum model disagreement;
- nominal entropy / information gain;
- decision-focused EVSI using the existing kernel;
- robust / worst-case decision value across the registered model set;
- clairvoyant observation choice as diagnostic ceiling only, not a competitive baseline.

A measurement that is high-information but cannot change an action should not win a decision-value tournament. A measurement that distinguishes two plausible models and reverses a consequential action should be rewarded according to declared loss and feasibility.

## 5. Feasibility before ranking

Candidate observations must first pass a feasibility gate:
- access and safety;
- landowner/community consent and applicable Indigenous / national governance;
- data purpose, retention, derivative and publication rights;
- seasonality, weather and field-team availability;
- survey cost, travel and crew-hours;
- measurement disturbance, sample destruction and wildlife disturbance.

An infeasible or unauthorized observation is excluded before EVSI ranking. High expected information value never grants permission to enter or intervene.

## 6. Metrics

Report separately:
- expected / realized EVSI calibration;
- realized regret reduction against the no-observation baseline;
- action-switch precision / recall;
- decision stability across plausible model structures;
- cost per unit of regret reduction;
- high-entropy observations with zero decision value;
- missed high-value observations;
- false escalation and unnecessary surveys;
- crew-hours, travel, delay and ecological disturbance;
- abstention / not-assessable rate;
- rights/access violations.

No universal composite score; hard-gate rights and safety failures.

## 7. Public corpus and future blind set

The companion JSON contains 20 public development cases across protection, restoration, disturbance verification, monitoring allocation, regional transfer, model-set inadequacy and observation-process uncertainty. Their expected dispositions are visible by design. They are not a blind holdout.

A future blind tournament requires an independently held generator / outcome set and a precommitted exact-head candidate run. Do not publish the hidden answers alongside a claimed blind evaluation.

## 8. Claim ceiling

Public corpus validity or synthetic EVSI agreement establishes only arithmetic / decision-policy behaviour for the declared toy cases. It does not establish optimal real conservation policy, field impact, legal authority or ecological calibration.


## 16 November 2026: decision tolerance feeds forecast usefulness

A forecast's error tolerance must be tied to its action context, not chosen to make the model pass. The new [#5485 usefulness-limit protocol](FOREST_FORECAST_USEFULNESS_LIMIT_V0_1.md) requires a unitful decision tolerance alongside proper-score skill, interval calibration, issuance coverage and block qualification.

For empirical use, the tolerance should be derived from the action-loss function or explicitly approved domain protocol already owned here. A biomass tolerance in Mg C ha⁻¹ cannot be reused for mortality probability, root turnover or recruitment. The forecast-usefulness adapter consumes a declared tolerance and reports the first failing horizon, but does not invent or optimize the real conservation action.
