# Membrane Service Evidence Benchmark — Industry/SOTA Comparison v0.1

**Status:** comparative design document; no field performance claim.

## Executive distinction

The commercial baseline is not a single ML classifier. Mature RO/NF operations use normalized performance tracking and manufacturer-specific cleaning criteria; low-pressure MF/UF integrity assurance uses separate continuous indirect and periodic direct integrity evidence; academic monitoring/prediction methods often use GRU, regularized XGBoost, CNN-GRU, PCA/statistical process monitoring and physics-informed models. The present programme is designed to compare against all of these layers without collapsing them into one score.

## Current reference points

| Reference | Published/industry practice | What it legitimately establishes | Our benchmark response |
|---|---|---|---|
| DuPont FilmTec RO/NF technical manual (public manual retrieved 2026) | Normalize permeate flow and salt passage against reference conditions; record normalized performance daily; manufacturer cleaning indicators include ~10% normalized permeate-flow decline, 5–10% normalized salt-passage increase, and 10–15% normalized stage pressure-drop increase. Temperature, salinity, pressure, recovery and pretreatment changes must be separated from fouling. | A practical RO/NF operating baseline and alert/cleaning reference. Not universal for UF, not a causal policy effect, not an integrity test. | M0/M0B reproduce transport calculations at arithmetic and grouped-regression levels; later M3 should compare normalized metrics and thresholds only after the relevant plant tags and correction factors are acquired. |
| EPA 2025 AWOP low-pressure membrane optimization goals | For selected MF/UF contexts, recommends unit-specific indirect integrity monitoring using online turbidity or particle counts at ≤1-minute intervals; example goals include IFE turbidity ≤0.05 NTU or 1.0–3.0 μm particle counts ≤10/mL. These are optimization goals, not a universal legal limit. | A distinct quality/integrity monitoring layer at the individual membrane-unit level. | M2 preserves lab results at actual time/location/analyte only; the integrity benchmark must remain separate from flux/TMP and from sparse lab labels. |
| EPA / LT2 membrane-filtration guidance | Direct integrity testing and continuous indirect integrity monitoring are separate methods; a hydraulic fouling indicator does not certify barrier integrity. | A compliance/integrity evidence boundary. | No membrane-integrity or water-quality claim is allowed from the telemetry M0/M1 outputs. Direct/indirect integrity evidence needs its own dataset and protocol. |
| 2026 mechanism-aligned ML review (UF/NF/RO) | Discusses GRU for dense UF hydraulic signals, regularized XGBoost for multicollinear NF variables, and CNN-GRU/SHAP for heterogeneous RO data; stresses plant-scale validation, SCADA noise, sparse data, fallback control and edge deployment. | A useful model-family baseline ladder and current research agenda, not proof of generalization across plants. | Baseline ladder starts with schedule/operator threshold/EWMA-CUSUM/mechanistic transport, then XGBoost/GRU, then hybrid/fusion. Model complexity is gated by data structure and external validation. |
| 2026 drinking-water membrane review (78 studies in 2015–2024) | Finds monitoring and prediction substantially more mature than deployable control; persistent gaps include data comparability, temporal/external validation, sensor drift, missing data, decision thresholds, actuator constraints and plant automation. | A maturity-gap map across research evidence. | M1 measurement integrity and #5012 counterfactual CIP are separate workstreams, with abstention and human authority. |
| 2026 RO real-time sensing review | Proposes fouling-detection metrics including limit of detection (LOD), alarm lead time (ALT), false-alarm rate (FAR), and stability under temperature/salinity variation; connects responses to monitor, mitigate, prepare CIP and trigger CIP. | A useful detection-to-action reporting vocabulary. | Add LOD/ALT/FAR, sensor drift/stability and policy consequences to the eventual site benchmark. An alert is not an action authorization. |
| Armstrong et al. (2022) RO cross-flow transport corpus | Quantifies errors from simplifying concentration-polarization and rejection assumptions using pressure, flux and solute rejection. | A transport-method validation corpus, not long-horizon fouling/CIP telemetry. | M0 pointwise arithmetic plus M0B synthetic grouped pressure-series regression; exact paper reproduction remains blocked on authorized raw workbook bytes and method grouping. |

## What SOTA does not yet solve

Across the 2026 reviews, the most durable gap is not that no one has a more expressive neural model. It is the lack of common evidence boundaries between measured process state, sensor reliability, membrane integrity, fouling mechanism, and intervention outcome. Random-window accuracy cannot prove cross-plant generalization or safe cleaning decisions.

An honest competitive claim therefore needs matched baselines on the same process population, data availability, time horizon, operating domain and held-out sites/modules/events. Report calibration, abstention, missingness and worst-regime performance—not only average RMSE or PR-AUC.

## Proposed benchmark stack

1. **M0 / M0B transport fidelity:** unit-checked net driving pressure, concentration polarization, solute rejection, pointwise permeance and replicate-grouped pressure-series fit.
2. **M1 measurement integrity:** distinguish process movement from stuck, biased, missing, stale, out-of-domain and historian failure; measure false alarms per sensor-day and correct abstention.
3. **M2 sampled outcome alignment:** actual laboratory sample time/location/analyte/method/qualifier only; no interpolated quality labels.
4. **M3 membrane-state forecasting:** chronological UF/NF/RO-specific baselines, event detection only when independent event truth exists, and temporal/site/module holdouts.
5. **M4 mechanism qualification:** require direct supporting evidence (e.g., autopsy/chemistry/known treatment response); otherwise label indeterminate mechanism.
6. **M5 causal intervention value:** target-trial or other defensible causal estimation of CIP strategies with overlap, confounding, treatment-version and replacement/censoring checks.
7. **M6 service outcome frontier:** optimize profile-conforming water, integrity, availability, energy, chemicals, cleaning water, waste, membrane replacement and lifecycle cost together.

Do not collapse these into a single leaderboard number. Publish a Pareto frontier and a per-process scorecard.

## Proposed competitive targets (not achieved)

- Forecasting: beat persistence, trend, EWMA/CUSUM, normalized mechanistic and operator/SOP baselines at every registered horizon on untouched chronological holdouts.
- Fault detection: target ≥95% recall on registered synthetic fault families at ≤1 false sensor-fault alert per 30 sensor-days, then repeat with audited real labels.
- Abstention: ≥90% correct abstention on synthetic ambiguous/out-of-domain cases, then calibrate selective risk on external sites.
- Warning quality: report LOD, ALT distribution and FAR at fixed false-alert burden; do not optimize lead time by flooding operators with alarms.
- Decision value: target ≥10% lower lifecycle cleaning cost per m³ conforming permeate than the strongest scheduled/threshold policy at matched integrity/quality constraints, after causal evaluation.
- Service performance: target ≥10% more conforming permeate per cleaning cycle or ≥10% reduction in cleaning energy/chemical burden without worse integrity, availability or membrane-life outcomes.
- External validity: leave-one-site/module/feed-regime-out tests; no claim of generality from one installation.

These are research targets, not achieved performance.

## Evidence maturity ladder

| Level | Evidence | Allowed claim |
|---|---|---|
| L0 | source metadata only | dataset candidate catalogued |
| L1 | synthetic fixtures and arithmetic oracle | software/contract behavior on synthetic cases |
| L2 | authorized raw bytes, hashes, inventory, labels audited | retrospective dataset analysis |
| L3 | untouched chronological/site/module validation | externally held-out retrospective model performance |
| L4 | prospective shadow mode, no control authority | prospective alert quality and operator workload |
| L5 | operator-approved advisory study with predeclared guardrails | observed decision support impact |
| L6 | controlled, independently metered intervention trial under approved procedures | measured causal/process effect within tested scope |

Current membrane work is L1. The Bristol source files have not yet been acquired in this environment; the Morocco SCADA is request-only; the Armstrong repository file terms remain to be verified.

## Sources

- [DuPont FilmTec RO/NF Technical Manual](https://www.dupont.com/content/dam/water/amer/us/en/water/public/documents/en/RO-NF-FilmTec-Manual-45-D01504-en.pdf.)
- [US EPA, Water Quality Goals and Operational Criteria for Optimization of Low-Pressure Membrane Filtration (2025, 815-B-25-007)](https://www.epa.gov/system/files/documents/2025-10/water-quality-goals-and-operational-criteria-for-optimization-of-low-pressure-membrane-filtration.pdf)
- [Yang et al. (2026), Mechanism-aligned ML for pressure-driven membrane fouling, DOI 10.1016/j.jwpe.2026.110543](https://doi.org/10.1016/j.jwpe.2026.110543)
- [Arembage et al. (2026), Integrated approaches to membrane fouling monitoring, prediction and control, DOI 10.3389/frwa.2026.1863523](https://doi.org/10.3389/frwa.2026.1863523)
- [Aboelala et al. (2026), In-situ and real-time fouling monitoring in RO desalination, DOI 10.1016/j.desal.2026.120612](https://doi.org/10.1016/j.desal.2026.120612)
- [Armstrong et al. (2022), RO transport dataset, DOI 10.1016/j.dib.2022.108538](https://doi.org/10.1016/j.dib.2022.108538)