# Forest forecast usefulness limits under regime change v0.1

**Issue:** [#5485 — FOREST-FORECAST-USEFULNESS-LIMIT-001](https://github.com/Luminous-Dynamics/symtropy/issues/5485)  
**Composes with:** [FOREST probabilistic forecast benchmark](FOREST_PROBABILISTIC_FORECAST_BENCHMARK_V0_1.md), [block coverage inference](FOREST_BLOCK_COVERAGE_INFERENCE_V0_1.md), [block conformal benchmark](FOREST_BLOCK_CONFORMAL_BENCHMARK_V0_1.md), [decision-value tournament](FOREST_DECISION_VALUE_V0_1.md), [model-set adequacy](FOREST_MODEL_SET_ADEQUACY_V0_1.md)  
**Date:** 16 November 2026  
**Status:** synthetic known-truth policy; no empirical usefulness horizon

## 1. Paradigm: forecasting accuracy is not forecast usefulness

A forest forecast can retain a good pooled score while becoming useless for a particular management decision at a particular horizon. Conversely, a variable may remain predictable at a long horizon after a disturbance even when intermediate forecasts are poor. One universal horizon for an entire model is therefore scientifically meaningless.

We define a decision-specific forecast usefulness limit for each variable, regime and declared decision tolerance. It is not a new proper scoring rule. It is a qualification policy layered over existing CRPS, interval calibration, block evaluation and decision-value work.

The 15 October 2026 forestry digital-twin review defines operational digital twins through repeated observations, dynamic model-state/parameter updates, explicit uncertainty propagation and feedback to decision support. It also reports that most forest digital twins remain prototype or early operational, with tropical and boreal ecosystems underrepresented: https://doi.org/10.1007/s44564-026-00029-8.

The 14 August 2026 ecosystem model-ensemble review calls for observationally grounded model intercomparison diagnostics to drive model selection and for stakeholder/scientist co-design of decision-relevant metrics: https://doi.org/10.1111/gcb.70987.

## 2. Qualification policy

For one context (variable + unit + site/regime + decision tolerance), a horizon qualifies only when **every declared held-out block** meets every condition:

1. positive paired CRPS skill against a baseline selected before the evaluation outcomes are revealed;
2. central 80% interval coverage within the predeclared calibration band;
3. issuance fraction above the minimum, with every abstention retained in the denominator;
4. mean absolute median error no greater than the unitful decision tolerance.

The context's usefulness limit is the longest initial contiguous prefix of passing horizons. If 30 and 90 days pass, 180 fails, and 365 passes again, the reported limit remains 90 days. The later pass is still reported as a recovery, not allowed to erase the failure.

This v0.1 policy is deliberately conservative (all blocks must pass). It does not claim a confidence guarantee from four blocks. The block results are diagnostics; empirical uncertainty must use the separate block-coverage and dependence methods.

## 3. Synthetic cases

The machine-readable fixture has two four-block contexts, each evaluated at 30, 90, 180 and 365 days:
- **Stable canopy biomass:** 30/90 pass, 180 fails, and 365 recovers. The usefulness limit must be 90 days.
- **Post-disturbance mortality probability:** 30 passes, 90 fails from a biased forecast, 180 fails the issuance-coverage requirement due to abstention, and 365 recovers. The limit must be 30 days.

These are synthetic proof cases, not fitted models or observed forest data. Their values and decision tolerances are invented to test the policy.

## 4. Why this is stronger than a single pooled score

The receipt publishes for every horizon and block:
- eligible and issued forecast counts;
- issuance coverage;
- paired CRPSS;
- central 80% interval coverage;
- mean absolute median error;
- qualification and explicit failure reasons.

A model can therefore not hide a collapsed regime, high abstention or a decision-tolerance breach behind a favorable global average. The baseline identity is frozen in the input manifest. Horizons with absent block support return an error/NOT-ESTIMABLE path rather than PASS.

## 5. Empirical tournament protocol

Before evaluation, freeze the target variable, unit, observation operator, source/model digests, forecast issue time, horizon, baseline, site/regime labels, held-out blocks, issuance floor, interval tolerance and decision loss/tolerance. Evaluate with rolling-origin hindcasts and leave-site/leave-block splits. Keep pre-disturbance and post-disturbance performance separate.

For a real decision, the unitful tolerance must be derived from the decision loss in #5223 or an explicitly approved domain protocol. It cannot be tuned after seeing which model wins. The 80% interval band is a calibration diagnostic, not a formal confidence interval or coverage guarantee. The all-block rule is intentionally strict and may be replaced only by a preregistered uncertainty-aware policy with adequate independent blocks.

## 6. Proposed empirical targets (not achieved)

- Positive CRPSS lower confidence bound against a frozen strong baseline within each declared regime.
- 80%/95% interval coverage within prespecified tolerances with block-aware uncertainty.
- Decision-tolerance compliance at each horizon, not just a pooled error score.
- At least 90% forecast issuance at horizons claimed useful, while reporting the full risk–coverage trade-off.
- Useful-horizon limits evaluated on fully held-out sites/regions and post-disturbance regimes.
- Publish horizon-limit curves by variable, site, decision and regime, including NOT-ESTIMABLE zones.
- Never advertise a horizon beyond the first failed prefix point without separately qualifying a new forecast origin/regime.

## 7. Claim ceiling

Synthetic PASS validates the qualification policy and arithmetic only. It does not establish AFEX forecast skill, a forest's actual forecast horizon, calibrated disturbance recovery, or operational decision benefit. Empirical limits require admitted repeated observations, frozen rolling-origin forecasts and independent held-out blocks/sites.


## 8. As-of integrity is a prerequisite for an empirical usefulness limit

A forecast usefulness horizon is meaningful only if the model input set reflects what was actually known at issue time. The [#5487 as-of integrity gate](https://github.com/Luminous-Dynamics/symtropy/issues/5487) separates event/collection time from source/QA/rights/ingestion availability and freezes the exact data versions used by each forecast origin. Otherwise, retrospective lab results, later satellite composites or corrected plot data can leak into a historical forecast.

Empirical usefulness limits must therefore bind the information cutoff, issue time, source versions and fitted-transform data cutoff. The new synthetic gate is [here](FOREST_AS_OF_FORECAST_INTEGRITY_V0_1.md); it does not itself establish any AFEX historical availability timestamp.
