# Known-Truth Model-Set Diagnostics v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Model-set workstream:** [#5198](https://github.com/Luminous-Dynamics/symtropy/issues/5198)  
**Status:** five public synthetic cohort-space diagnostics; not blind and not ecologically calibrated

## 1. Why this executable slice exists

The 30-case model-set development manifest validates that ten omitted-mechanism families have three uniquely identified public variants apiece. It does not execute model forecasts or test detector behaviour.

This companion kernel starts the next step: execute small known-truth counterexamples where:
- candidate models fit the same historical aggregate biomass but imply different cohort structures;
- different demographic mechanisms alias under total biomass;
- all candidates share one structural assumption and therefore make the same incorrect prediction;
- matched and history-fit controls prevent every case from being labelled model inadequacy.

The kernel is deterministic and analytic. It does not implement forest growth, allometry, mortality hazards, recruitment, climate response or a calibrated ecological model.

## 2. Diagnostic inputs and outputs

Each scenario declares:
- observed historical total biomass;
- a synthetic size-resolved validation state (small and large cohort live biomass);
- history-fit, structural-disagreement and common-omission thresholds;
- at least one candidate model identity, its historical total prediction and its future size-resolved prediction.

The evaluator calculates:
- whether every candidate fits the historical aggregate within tolerance;
- maximum pairwise L1 distance between candidate size-resolved forecasts;
- minimum and maximum L1 residual against the validation state;
- a conservative diagnostic disposition.

Precedence is:
1. **History fit failure:** at least one candidate fails the declared historical aggregate tolerance.
2. **Structural disagreement:** candidates fit history but their size-resolved future forecasts diverge above threshold.
3. **Common structural omission detected on validation:** candidates fit history and agree with each other, but all are far from the validation state.
4. **No failure detected on this synthetic case:** candidates fit history, are not structurally divergent, and are within the declared validation residual threshold.

The fourth disposition is not a model-set adequacy certificate. It means only that this one known-truth control did not falsify the candidates under the declared variables and tolerance.

## 3. Five worked counterexamples

The public manifest is `cases/forest-model-set-known-truth-diagnostics-synthetic-v0.1.json` and is evaluated by `validator/src/model_set_kernel.rs`.

| Case | Candidate outputs | Known validation state | Expected diagnostic |
|---|---|---|---|
| Growth–mortality compensation | (40,60) versus (60,40), both total 100 | (50,50) | Structural disagreement; same aggregate cannot resolve cohort composition |
| Recruitment–growth aliasing | (20,90) versus (10,100), both total 110 | (15,95) | Structural disagreement; cohort-space observation matters |
| Delayed-response common omission | Both candidates predict (80,20) | (100,0) | Both candidates share the same miss; common omission detected on validation |
| Matched control | Both predict (45,55) | (45,55) | No failure detected on this synthetic case |
| Historical-fit failure | Candidate history totals 90 and 100 against observed 100 | (45,55) | Candidate history-fit failure, not an omitted-mechanism label |

Values are synthetic biomass units. They are chosen for hand calculation and are not measured values, fitted parameters or species-level responses.

## 4. Benchmark interpretation

This is a tiny diagnostic kernel, not a competitive model detector. It demonstrates how to avoid two errors:

- **Aggregate-fit fallacy:** a small total-biomass residual does not imply size-structure or process adequacy.
- **Agreement fallacy:** all candidates agreeing does not imply adequacy when an independent validation observation contradicts all of them.

It does not yet test:
- blind generator custody;
- model fitting or parameter calibration;
- time-series process simulation;
- probabilistic forecasts or CRPS;
- intervention counterfactuals;
- omission recall, false-alarm rate, selective risk or decision regret;
- model-set adequacy on real forest observations.

A genuine blind challenge still requires an independent custodian, precommitted model registry and predictions, sealed future observations or generator identities, and leakage-resistant evaluation. The 30-family public manifest remains development data; this five-case diagnostic publishes its known truth and is deliberately not blind.

## 5. Claim ceiling

A successful `PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS` receipt establishes that the toy candidate outputs produce the declared synthetic diagnostics. It does not prove universal mechanism detection, ecological model adequacy, a tropical forest forecast, cross-region transfer, SOTA performance or conservation impact.
