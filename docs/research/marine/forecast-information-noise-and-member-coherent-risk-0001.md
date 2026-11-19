# Marine Forecast Information, Noise and Coherent-Risk Evaluation

**Status:** research protocol, not a validated algorithm or operational capability  
**Date:** 2026-11-19  
**Tracks:** Symtropy #6220 (forecast scorecard), #6233 (forecast credibility and coherent ensemble risk)

## 1. Why a conventional scorecard is insufficient

RMSE, MAE, CRPS and Brier score answer important but different questions. A lower aggregate error does not necessarily mean the forecast retains more information about where and when a dangerous event will occur. Smoothing can reduce RMSE while suppressing forecast variability; post-processing can improve reliability without creating intrinsic predictive resolution.

Bonavita and Geer (2026) formalize this distinction using forecast information (FI), information error (IE) and noise error (NE), separating intrinsic resolution from reliability/calibration. Their practical formulation debiases anomalies and applies area weighting. They show why conventional scores can reward smoothing and why information/resolution should be reported separately.

References:
- Bonavita, M. & Geer, A. (2026), *Forecast verification using information and noise*, Quarterly Journal of the Royal Meteorological Society, DOI: https://doi.org/10.1002/qj.70109
- ECMWF explanation (25 March 2026): https://www.ecmwf.int/en/about/media-centre/science-blog/2026/separating-signal-noise
- ECMWF Technical Memorandum 927: https://www.ecmwf.int/en/elibrary/81662-forecast-verification-using-information-and-noise

## 2. Information/noise diagnostic to add

For one forecast field and one verifying field on a fixed common support, define bias-corrected anomalies relative to a pinned climatology. Let (d_f) and (d_t) be forecast and truth anomalies after subtracting their respective weighted regional mean anomalies. Let (w_i) be normalized support weights, with (sum_i w_i=1).

```text
SDAF = sqrt(sum_i w_i * d_f[i]^2)
SDAV = sqrt(sum_i w_i * d_t[i]^2)
p    = sum_i w_i * d_f[i] * d_t[i]
ACC  = p / (SDAF * SDAV)
FI   = p / SDAV^2 = (SDAF / SDAV) * ACC
IE   = abs(1 - FI) * SDAV
NE   = sqrt(sum_i w_i * (d_f[i] - FI * d_t[i])^2)
STDE^2 = IE^2 + NE^2
```

This is a field/cycle diagnostic, not a per-row scalar metric. Preserve signed FI (it can be negative or exceed one). Degenerate verification variance (SDAV near zero) makes FI undefined and must produce an incomplete/undefined result, not a fabricated zero. Keep the per-field/per-cycle decomposition intact before aggregating over cycles: averaging first breaks the exact per-cycle orthogonal decomposition.

### Marine support and weighting rules

1. Use fixed, pre-registered verification support for model-to-model comparisons. A route must not select its own verification cells after seeing the forecast.
2. Keep native-grid, common-grid and point-observation evaluations separate. NDBC/buoy stations are sparse local checks, not a route-wide truth field.
3. For regular latitude/longitude fields, use normalized cell-area weights (latitude-dependent); preserve grid/CRS/interpolation and mask digests.
4. If an additional route-exposure weighting is useful, report it as a separate decision-weighted scorecard. Do not silently replace the area-weighted field-verification definition with weights selected by the planner.
5. For wind, compute vector diagnostics on (u/v) components and retain speed/direction diagnostics separately; circular direction error must handle calm-wind cases explicitly.
6. For wave variables, keep significant height, period and direction separate and report threshold/event diagnostics (e.g. steep/large wave conditions) rather than blending unlike physical units.

## 3. Smoothing and calibration falsification controls

Each held-out cycle should include paired rows for:
- raw deterministic control/member forecast;
- ensemble mean;
- a pinned, trained calibration or bias-correction transform;
- a controlled smoothing/damping transformation;
- persistence and climatology;
- the strongest information-matched deployable operational source.

Report RMSE/MAE/CRPS/Brier alongside FI/IE/NE, forecast activity (SDAF), observed activity (SDAV), ACC, threshold reliability and event discrimination. The smoothing negative control should demonstrate whether RMSE improves only because activity is damped while information/resolution worsens. Calibration data and transforms must be fitted only on development data, selected on validation, and frozen before test.

## 4. Weather-blind policy is a mandatory decision baseline

The 15 October 2026 *Ocean Engineering* paper on risk-aware ship weather routing with adaptive hexagonal grids reports a critical failure mode: optimizing on a failed deterministic forecast produced more exposure than weather-blind routing. Its abstract also reports that member-coherent CVaR captured realized exposure missed by marginal summaries. The paper combines adaptive multi-resolution hexagonal routing, Traffic Separation Scheme topology, edge-level intact-stability pruning and an ensemble of 50 perturbed ECMWF members.

Reference: Zhang et al. (2026), DOI https://doi.org/10.1016/j.oceaneng.2026.127661

This yields two benchmark rules:

- Weather-blind policy is a first-class matched policy arm, not an informal fallback.
- Weather-informed policies must be evaluated against weather-blind outcomes in forecast-failure controls, by hazard regime and lead time. A poor forecast must not be allowed to make the system look better merely because it supplies more detailed inputs.

## 5. Preserve coherent ensemble paths

For ensemble member (m), preserve the same member identity and spatial-temporal field through every route edge and decision instant. Simulate one whole-voyage exposure/loss (L_m) per coherent member path; calculate exceedance probability and CVaR from those path-level losses.

Do not independently sample each edge, time step or hazard marginal. Such resampling can destroy correlation and create physically incoherent voyages. A replay receipt should bind:
- ensemble source, cycle, member ID and member artifact digest;
- forecast lead/valid time and first-availability receipt;
- interpolation/regridding transform and route-edge mapping;
- mission/craft/service fingerprint and exact policy input set;
- per-member route-level exposure vector and loss calculation version.

Report both member-coherent CVaR and marginal summaries as a diagnostic contrast. The marginal result is not a substitute for coherent-path risk.

## 6. Current industry and research comparison

- **Weather verification:** WeatherBench 2 emphasizes a portfolio of metrics rather than a single forecast score. It is a useful methodology reference, but its headline global atmospheric fields do not directly benchmark nearshore sailing decisions. https://sites.research.google/weatherbench/
- **Ship routing:** Mahmoodi et al. (March 2026) compare ensemble-mean, worst-case, risk-aware and CVaR objectives using onboard ship data and a learned fuel model. Their source abstract says data are available on request, so independent data reproduction may be limited. DOI: https://doi.org/10.1016/j.jii.2026.101075
- **Recent marine SOTA:** Zhang et al. (October 2026) integrate adaptive hexagonal graphs, traffic separation, edge-level stability pruning and 50-member ensemble risk. The abstract reports 13–48x fewer nodes than a uniform-fine graph at comparable route quality in its cases; this should be treated as a paper-reported comparator, not as a result reproduced here. DOI: https://doi.org/10.1016/j.oceaneng.2026.127661
- **Commercial sailing product:** PredictWind's AI Router says it reads eight model routes and summarizes demanding periods and confidence. Its AI Polars are a proprietary five-dimensional dataset that the vendor says cannot be exported. This makes an authorized black-box trial useful, but it does not create an independently reproducible baseline unless inputs/settings can be matched. https://www.predictwind.com/features/ai-weather-router and https://help.predictwind.com/en/articles/8666575-datahub-can-i-download-ai-polars

## 7. Proposed benchmark ladder

| Level | Benchmark | Acceptance target |
|---|---|---|
| Replay integrity | Source time, receipt digests, split identities, rights | 100% admitted records bind exact source/cycle/lead/availability and fold; zero future leakage |
| Score correctness | Deterministic, CRPS, Brier, reliability, FI/IE/NE | 100% agreement with independent known-answer fixtures within preregistered floating-point tolerance |
| Resolution vs smoothing | Raw, ensemble mean, calibrated and deliberately damped controls | Report RMSE and FI/IE/NE together; detect any RMSE-only win that loses information |
| Forecast value | Weather-informed vs weather-blind and persistence/climatology | Zero unreported increases in hazard exposure on frozen failed-forecast controls |
| Tail-risk integrity | Per-member route path and mission loss | 100% member identity continuity; independent evaluator reproduces CVaR |
| Mission value | Energy, arrival regret, service continuity, water/food/reserves, workload, comfort | Earlier 10% energy and CVaR95 improvement goals remain research targets; require paired whole-voyage/weather-event intervals and no hard-gate regression |
| Search efficiency | Adaptive vs uniform/coarse grid on identical ODD and constraints | Publish quality/compute Pareto curves; compare against paper-reported 13–48x node reductions only under matched test conditions |

## 8. Non-negotiable evidence boundaries

- Reanalysis, hindsight and perfect-foresight references are evaluator-only.
- A forecast is deployable only when first availability is proven no later than the decision cutoff.
- A digest identifies bytes; it does not authenticate the publisher or validate physical truth.
- A valid scorecard does not establish statistical power, operational safety, seaworthiness or regulatory compliance.
- No scalar reward may compensate for failed stability, safety, essential-service or comfort hard gates.
- Statistical claims require prospective power analysis and paired whole-voyage/weather-event block resampling, not adjacent-timestep bootstrap.
