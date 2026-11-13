# Membrane Measurement Integrity M1 — Sensor/Process Separation v0.1

**Tracking issue:** [#5035](https://github.com/Luminous-Dynamics/symtropy/issues/5035)
**Programme:** [#4977](https://github.com/Luminous-Dynamics/symtropy/issues/4977)
**Dataset intake:** [#4996](https://github.com/Luminous-Dynamics/symtropy/issues/4996)

## Why this is separate

A pressure or flux change can mean process degradation, a sensor fault, a historian defect or a changed operating regime. These are not interchangeable labels. Missing, stale, uncalibrated or invalid measurements must not be interpreted as normal operation, and hydraulic proxies do not establish membrane integrity or permeate quality.

Recent 2026 review literature identifies SCADA noise, data sparsity, sensor drift, missing-data treatment, external validation, interpretability and plant integration as barriers to deployable membrane process control. DuPont's February 2026 FilmTec RO/NF technical manual recommends normalization of permeate flow and salt passage against reference conditions and daily recording. It gives manufacturer cleaning indicators involving normalized permeate-flow decline, normalized salt-passage rise and stage pressure-drop rise. These indicators are configuration-specific and must not be transferred as universal UF or RO thresholds.

EPA guidance treats continuous indirect integrity monitoring separately from direct integrity tests. A hydraulic fouling estimator must not be presented as a membrane-integrity test.

## State model

Keep four dimensions orthogonal:
1. Process condition: stable, coherently changing, degraded candidate, or out of domain.
2. Measurement condition: healthy, stuck-at suspected, missing, stale, inconsistent or invalid.
3. Evidence sufficiency: sufficient, partial, or insufficient for the inference being made.
4. Service/integrity condition: independent evidence from integrity testing and actual quality measurements; never inferred from flux/TMP alone.

M1 is a deterministic contract benchmark for distinguishing selected signal pathologies from coherent process movement. It is not a production detector and uses synthetic fixtures only.

## Synthetic test families

Eight fixtures exercise healthy stable telemetry, coherent hydraulic movement, stuck-at flux while TMP and derived permeability move, channel dropout, simultaneous historian outage, cross-channel measurement inconsistency, values outside a declared synthetic envelope, and an ambiguous combination that must abstain.

The benchmark does not call any coherent hydraulic change fouling. Its process conclusion is PROCESS_SHIFT_COHERENT. Suspect measurement channels are named only where the controlled fixture provides evidence; ambiguous combinations become INDETERMINATE.

## Data contract for real Bristol telemetry

The Bristol 34-month record is described as 5-minute rolling averages with flow, discharge pressure, filtration flux and UF permeability; metadata does not confirm TMP, temperature, calibration history or cleaning event labels. The 12-week Bristol trial separately describes UF TMP plus 5-minute telemetry and nine lab sample times at three treatment-chain locations.

- Do not temperature-normalize the 34-month permeability unless temperature and the correction basis are confirmed in the acquired files.
- Do not invent TMP or a cleaning/fouling label for the long-run telemetry.
- Do not derive flux from flow unless membrane area and unit provenance are verified.
- Do not treat 5-minute rolling averages as raw transient/high-frequency data.
- Do not impute sensor gaps as normal; preserve missingness and data-quality flags.
- Do not classify membrane integrity or water quality from hydraulic channels.
- Fit thresholds/calibration only on training/calibration periods, never the final temporal holdout.

## Evaluation ladder

1. Schema, timebase, unit and calibration-provenance checks.
2. Missingness, freshness and historian-gap accounting.
3. Flatline/stuck-at detection conditioned on changing independent peer channels.
4. Cross-channel physical consistency only when variables are synchronized, units known and all required inputs available.
5. Robust EWMA/CUSUM and change-point baselines.
6. Probabilistic state estimator with uncertainty, domain-shift detection and abstention.
7. Mechanism-aligned GRU/XGBoost/CNN-GRU only after population size and signal structure justify it.

Report per-process and per-site performance; never pool UF and RO headline scores.

## Metrics and stretch goals (not achieved)

- Fault-family precision/recall and false fault alerts per sensor-day.
- Coherent process-shift detection delay and false change points.
- Missing/stale exposure and fraction of operation in unsupported state.
- Selective risk versus coverage; uncertainty calibration under drift and dropout.
- Correct abstention on ambiguous/out-of-domain cases.
- Stretch goal: at least 95% recall on registered synthetic fault families at no more than one false fault alert per 30 synthetic sensor-days.
- Stretch goal: at least 90% correct abstention on ambiguous/out-of-domain synthetic cases.
- Zero false fouling-mechanism labels in sensor-fault negative controls.

These goals are not field performance claims.

## Safety boundary

M1 emits diagnostic/advisory states only. It cannot authorize CIP, chemical dosing, bypass, valve action, pressure setpoint changes, product release or autonomous control. Loss of integrity/quality evidence is not evidence of integrity. Unknown or stale observations mean abstain.

## Nonclaims

No plant sensor is diagnosed; no fouling mechanism is identified; no normalized RO metric is computed; no UF/RO operating limit is established; no water quality, membrane integrity, controller or plant is qualified.