# Forest Model-Set Adequacy Challenge v0.1

**Tracking:** [FOREST-MODEL-SET-ADEQUACY-001 #5198](https://github.com/Luminous-Dynamics/symtropy/issues/5198)  
**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Composes:** #2244 model structure uncertainty, #2334 model-set adequacy, #2186 causal validity, #2214 ecological transfer, #2171 active sensing  
**Status:** public case-family contract; not a sealed blind benchmark

## 1. Purpose

A system can be well calibrated within every registered model and still be wrong because all candidate models omit the same ecological mechanism.

This challenge evaluates whether a modeller:
- distinguishes parameter uncertainty from structural disagreement;
- detects when registered models are collectively inadequate;
- resists observation-process shifts that masquerade as ecological change;
- chooses useful and feasible follow-up measurements;
- abstains when a decision depends on unresolved model structure.

This document defines a forest-specific specialization. The generic semantics, evidence and model-set adequacy authority remain in Symtropy #2244 and #2334.

## 2. Public versus blind challenge data

The public case-family JSON is a **development / conformance manifest**. It is not a blind test and contains no hidden generator equations, hidden parameters or future outcomes.

A future blind tournament must:
1. freeze and hash the candidate model registry, training data, candidate generator families, evaluator and scoring rules;
2. place the held-out generator identities/parameters and evaluation outcomes with an independent evaluator or separately controlled custody boundary;
3. disclose only the permitted historical observations and action catalogue to the candidate;
4. commit the candidate's forecasts, adequacy status and experiment choice before revealing the held-out result;
5. release the independent receipt and, where rights allow, the complete case after the challenge closes.

Publishing the hidden generator in the same public repository as the challenge would invalidate blind evaluation. A manifest hash is not a substitute for independent custody.

## 3. Model uncertainty categories

Keep at least five distinct sources of uncertainty:

| Category | Example | Required treatment |
|---|---|---|
| Parameter uncertainty | mortality rate or shade response differs within one structural model | propagate parameter distribution / sensitivity |
| Within-model stochasticity | recruitment or disturbance sampling varies between runs | repeat independent seeds and retain the stochastic model |
| Observation uncertainty | plot measurement error, GEDI prediction SE, uncertain species label | source-specific observation operator |
| Structural disagreement | cohort and spatial-canopy models predict different gap recovery | preserve model identities and per-model outputs |
| Model-set inadequacy | all candidate models omit a lagged mortality or seed-limitation mechanism | adequacy challenge, additional experiment, wider claim or abstention |

Ensemble agreement addresses only agreement within the declared set. It cannot establish that the set contains all relevant mechanisms.

## 4. Case families

The public corpus defines ten families:
The current public development manifest contains **30 uniquely identified variants (three per family)**. These cases check that the family contract is represented consistently; they do not execute a candidate model-set detector, disclose hidden generator truth, or demonstrate omitted-mechanism detection. The corpus remains public and is deliberately not blind.


1. Growth–mortality compensation: similar net biomass histories but different stocks / size structure.
2. Recruitment–growth aliasing: similar total biomass but different cohort distributions.
3. Delayed response: instantaneous candidate response versus lagged recovery / mortality in the evaluator's held-out generator.
4. Size-dependent mortality: aggregate hazard fit hides a size-class response.
5. Seed / propagule limitation: canopy gap and light alone are insufficient to explain recruitment.
6. Spatial canopy topology: equal mean LAI but different gap structure; compose #5196.
7. Disturbance–climate interaction: separate single-shock fits conceal a combined mechanism.
8. Observation-process shift: unchanged latent ecology but changed census / footprint / sensor / missingness process.
9. Cross-region parameter transfer: a source-region model is applied to a target-region regime with different trait / soil / climate support.
10. Common structural omission: every registered model agrees, while blind truth contains a mechanism absent from all.

Parameters and mechanisms remain synthetic until an independent source and process model justify a real-world interpretation.

## 5. Fair tournament

Freeze for every run:
- model registry, exact versions, process assumptions and parameter ranges;
- training observations and time cut-off;
- rights, source versions, units and observation operators;
- target variable, spatial and temporal support;
- validation and blind-test grouping;
- compute, wall-clock, calibration and experiment budgets;
- score definitions, thresholds, uncertainty method and stop rule.

Compare:
- strongest single model selected on a validation set;
- ensemble mean/median;
- model averaging that preserves each model identity;
- robust/minimax decision across the registered set;
- residual / posterior-predictive adequacy diagnostics;
- an adequacy-aware system that may abstain;
- random, nominal information-gain and decision-value / EVSI experiment selection;
- a clairvoyant oracle as a diagnostic upper bound only, never a leaderboard competitor.

The selected experiment must satisfy permissions, access, method, disturbance and cost constraints before it is ranked. High information gain cannot create field authority.

## 6. Metrics

Report separately:
- omission detection recall by generator family;
- false-adequacy rate on cases with adequate model sets;
- false-alarm rate on in-set but noisy observations;
- selective risk versus coverage and abstention;
- CRPS / Brier / log score where probabilistic outcomes support them;
- prediction interval coverage and width;
- model disagreement by state variable, horizon, site and regime;
- realized decision regret under the independent generator;
- experiment discrimination and expected/realized value;
- total resource cost, latency, compute and ecological disturbance;
- rights, source-provenance, split-leakage and hidden-truth violations.

No one weighted score can hide false adequacy, rights failure or invalid negative evidence.

## 7. Adversarial rules

- An ensemble mean cannot hide model disagreement.
- Agreement cannot be scored as adequate when all models share an omitted mechanism.
- Observation-process shifts must not become ecological change.
- A single census cannot become growth or mortality rate without matched repeated observations.
- Public development fixtures cannot be called blind holdouts.
- Hidden generator IDs, future state or evaluator oracle queries invalidate the run.
- Missing model / metric output is incomplete, not zero.
- Infeasible or unauthorized observations are filtered before information-value ranking.
- OOD and unresolved cases may legitimately return NOT_ASSESSABLE / ABSTAIN.
- Synthetic success is contract consistency, never universal mechanism completeness.

## 8. Connection to the forest pilot

The Amazon RAINFOR EIDC 2017–2019 dataset can support a structural/stand-state case once its plot supports and prior-census availability are checked. It is not automatically a temporal demographic panel. DRC and PNG national inventories can support regional stratification and structural validation with the access/missingness conditions in the dataset candidate register. The forest demographic tournament must not claim growth/mortality forecasting until compatible repeated plot observations are admitted.

## 9. Claim ceiling

The public manifest and its validator establish only that the challenge case definitions and declared expected dispositions are internally coherent. A future blind challenge can estimate sensitivity to its declared withheld mechanism families. Neither result proves that every ecological mechanism has been anticipated or that an ecosystem model is empirically valid.


## November 2026: public development versus sealed evaluation

The public development corpus remains intentionally public and must not be scored as a blind benchmark. A separate custody protocol and machine-readable candidate/receipt schemas are now tracked in [#5328 — FOREST-BLIND-ADEQUACY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5328):

- [Sealed model-set custody protocol](FOREST_BLIND_MODEL_SET_CUSTODY_V0_1.md)
- [Candidate submission schema](schemas/forest-blind-candidate-submission-v0.1.schema.json)
- [Sealed evaluation receipt schema](schemas/forest-blind-evaluation-receipt-v0.1.schema.json)

The protocol requires a separately controlled custodian, secret salt, a domain-separated SHA-256 commitment, immutable candidate submissions, separately signed evaluator receipts and a retained sealed reserve. No custodian or evaluator has been appointed; no blind challenge has run. These files specify a protocol, not an operational custody service.

The hidden generator must include counterfactual pairs that distinguish latent ecological change from observation-process change, plus false-alarm controls for parameter/stochastic variation. The public repository must never contain the hidden generator, seed, truth bundle or commitment opening while the candidate is being evaluated.
