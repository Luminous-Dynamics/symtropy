# Lunar excavation model benchmark contract — v0

This directory holds a run-level JSON Schema and an intentionally synthetic example. It is a data contract for issue [#6012](https://github.com/Luminous-Dynamics/symtropy/issues/6012), not a DEM/CEL solver and not a lunar-performance claim.

## Files

- `benchmark-run.schema.json`: JSON Schema Draft 2020-12 for one experiment/model-comparison record.
- `dataset-manifest.schema.json`: JSON Schema for group-disjoint partition assignments and blind-test freeze policy.
- `examples/synthetic-calibration-run.json`: explicitly synthetic calibration run fixture.
- `examples/synthetic-validation-run.json`: explicitly synthetic validation run fixture.
- `examples/synthetic-blind-run.json`: explicitly synthetic blind-test run fixture.
- `examples/synthetic-dataset-manifest.json`: three-partition, group-isolated synthetic manifest.
- `examples/synthetic-force-trace.csv`: tiny synthetic force trace referenced by the fixture.
- `examples/synthetic-model-config.json`: fixture model configuration.

## Why this contract exists

The 2026 literature makes a single headline error number an unsafe comparison:

- Sun et al. report below-5% relative error for a calibrated Hertz–Mindlin-with-bonding DEM on their matched JLU-H excavation and cone-penetration conditions. The paper explicitly distinguishes compacted simulant response from loose material; that result is a reproduction target for matched conditions, not a universal lunar-soil guarantee.
- Long-Fox et al. (2026) report broad force-trend agreement in CEL excavation simulations while identifying mean-force overprediction and unresolved strain-softening / consolidation-hardening behaviour.
- Long-Fox, Dotson and Britt (31 August 2026) review the dependence of simulant response on particle size, composition, preparation, compaction, temperature, pressure and environment, and recommend standardized testing, open property databases and larger-scale test facilities.
- MoLES3's 2026 ESA LUNA analogue campaign decomposes excavation, transfer and beneficiation, reporting transfer loss as the dominant recoverable inefficiency for its tested separated architecture. Its metrics demonstrate why a successful cut is not the same as delivered feedstock.

A benchmark therefore needs to preserve material batch, preparation, tool geometry, environment, operational regime, raw traces, instrument calibration, solver revision, parameters, uncertainty and output artifacts as first-class fields.

## Partition and leakage policy

Every record declares one partition: `calibration`, `validation`, or `blind_test`. The dataset-manifest schema requires four core isolation axes and can additionally promote laboratory and/or campaign identifiers when those axes are part of the study design:

1. material batch;
2. preparation / compaction protocol;
3. tool geometry;
4. operating regime (including static, continuous, vibratory and percussive families);
5. optionally, laboratory;
6. optionally, campaign.

Do not randomly split repeated samples from the same batch/run campaign and then call the result out-of-distribution generalization. The dataset manifest maps every group identifier on each isolation axis to one partition, so a given group cannot be assigned to two partitions within that axis. Keep blind-test labels and raw outcomes outside the model-facing environment until the candidate has frozen its executable, source revision, configuration, parameters, random seed and predictions. Repeats from one specimen or campaign stay in one partition. The CI guard checks the synthetic run-to-manifest join and the four core plus optional lab/campaign group assignments; a general evaluator must still validate every imported run against a real manifest and verify that the sealed blind data were not exposed.

## Measurement and artifact rules

- SI units are mandatory in field names and explicit in every measurement.
- `unknown` is represented by null value and null uncertainty; it is not zero.
- `measured`, `published`, and `estimated` observations must retain their source artifact. Physical measurements additionally require a calibration record for the measuring instrument.
- Every artifact path resolves to bytes whose SHA-256 equals the declared digest. IDs are unique within a run, and all *_artifact_id references resolve to exactly one artifact.
- Preserve the raw force-time series; peak/mean/impulse summaries do not replace it.
- Keep observed measurements separate from model predictions and prediction intervals.
- Do not convert simulant measurements into lunar-regolith facts or transfer bulk-regolith DEM calibration to competent-rock tunnelling.

The schemas check run/manifest shape. The fixture CI checks the example's group-to-partition assignments and required axes. Cross-file constraints (run-to-manifest agreement for every imported record, ID/reference integrity, time-series unit consistency, interval ordering, calibration validity/freshness, repeat leakage, source licensing, blind-set custody and digest verification) remain independent evaluator responsibilities and must fail closed.

## Benchmark protocol

### Gate 0 — provenance and admissibility

Reject a physical record if material batch, preparation, operating regime, instrument identity/calibration, raw artifact digest, or unit convention cannot be traced. Unknown values remain unknown. Keep synthetic and published records in separate evidence classes.

### Gate 1 — matched reproduction

Reproduce at least one published JLU-H DEM test and one JSC-1A CEL/static or percussive case only where the raw protocol, tool geometry, material preparation and measurements are actually available. Match test boundaries and reproduce the source's own metrics before making comparative claims. Do not reverse engineer a target from a plotted curve and then describe it as a fully matched reproduction.

### Gate 2 — blind generalization

Freeze disjoint holdouts for one or more of: unseen material batch, packing/compaction state, confining pressure, tool shape/edge wear, excavation depth, feed speed, dynamic/percussive mode, environment or laboratory. Report per-axis and joint holdout performance. A single aggregate score cannot hide a catastrophic regime failure.

### Gate 3 — model-form comparison

Run DEM, CEL/FEM, continuum, and analytical/empirical baselines on the same inputs. Report each solver's residuals, non-convergence and runtime. Preserve disagreement; do not average outputs into an untraceable pseudo-consensus. Specialist solvers retain physical-model authority.

### Gate 4 — system-level transfer

Connect excavation to collection, transfer, screening/beneficiation, storage and delivery. Measure loss at every material interface, energy per delivered mass, throughput per operating hour, jam/maintenance burden and recovery. The functional unit is accepted delivered material of a specified particle-size/quality class—not nominal cut volume or bucket payload.

## Metrics (report individually; no opaque leaderboard scalar)

**Physics prediction**
- time-aligned force-curve normalized RMSE and error in peak, mean and impulse;
- displaced-volume and as-built-geometry error;
- force/energy per displaced volume, with measured boundary;
- particle-flow / heave discrepancy where independently observed;
- model convergence/failure frequency.

**Uncertainty and generalization**
- empirical coverage of nominal 50%, 80%, 90% and 95% prediction intervals;
- interval width / sharpness and calibration residuals;
- error by batch, packing, pressure, geometry, depth, speed and operation mode;
- calibration-to-blind generalization gap and worst-regime error;
- transfer penalty between batches and laboratories.

**Experiment and compute efficiency**
- number, cost and duration of physical calibration experiments;
- compute time and energy per prediction, hardware and solver version;
- repeatability across runs and determinism where the solver claims it;
- recovery from numerical failure and percentage of runs marked unknown / invalid.

**Integrated excavation service**
- kWh per accepted delivered tonne and per accepted delivered m³ (separate units);
- transfer efficiency and mass closure at each interface;
- accepted delivered tonnes per mission-hour;
- downtime shares: cutting, collection, transfer, processing, maintenance, thermal/environment waits and recovery;
- wear per accepted delivered tonne, when actual wear measurements exist;
- autonomous fault detection, safe-stop and recovery time.

## Proposed improvement targets (hypotheses, not current results)

Use the best matched published or independently reproduced baseline on each test family. Then preregister candidate targets:

1. **Reproduction:** reproduce source metrics within the uncertainty and repeatability of the source protocol before tuning for blind data.
2. **Generalization:** on identical group-disjoint test partitions, reduce held-out normalized force-curve error by at least 20% versus the strongest predeclared baseline in the full tournament—including published XGBR, not merely an analytical equation—while maintaining nominal 90–95% interval coverage within a preregistered tolerance. Reproduce the published XGBR result on its matched source protocol before making a direct comparison. This is a research target to test, not a claim of present capability.
3. **Calibration efficiency:** reach the same held-out error and interval coverage with at least 25% fewer physical calibration runs than the strongest baseline. Count total specimens, operator-hours and test energy, not only solver iterations.
4. **Robustness:** no hidden pooling across material batches; publish worst-regime error and all non-converged cases. Any critical regime failure blocks a broad-model claim.
5. **Material chain:** close measured mass balance within combined instrument uncertainty or return `indeterminate / reconciliation required`; no loss is silently set to zero.
6. **Service productivity:** improve accepted delivered tonnes per mission-kWh and per mission-hour without reducing particle-size quality, availability, tool life or safety margins.

The percentage targets above are deliberately comparative. Absolute tolerances must be set after a matched terrestrial analogue pilot and tied to the instrument uncertainty and intended use.

## SOTA / industry comparison boundaries

- **DEM:** Sun et al.'s below-5% result is a matched JLU-H calibration/validation outcome; it is not a universal benchmark across batches or lunar conditions.
- **CEL:** Long-Fox et al.'s 2026 JSC-1A results show that force-trend reproduction does not by itself establish unbiased mean force or robust percussive response.
- **Simulant engineering:** the August 2026 review argues for consistent preparation/testing protocols, high-fidelity material characterization and public property data.
- **Lunar construction:** NASA's 2024 technology-status presentation described ISRU excavation prototypes at TRL 5/6 with a 10-tonne / 15-day pilot profile and limited autonomy; it separately said extraterrestrial excavation for construction had not yet been attempted. These figures are a dated programme snapshot, not a 2026 fleet benchmark.
- **Terrestrial TBM:** industry distinguishes penetration rate while cutting from utilization and calendar advance. USACE EM 1110-2-2901 reports project-average TBM utilization typically 35–50% and a historical database average of 38%; these are terrestrial rock-tunnelling references, not lunar predictions.
- **Integrated regolith handling:** MoLES3 shows why transfer and beneficiation boundaries must be measured separately from excavation.

The credible competitive claim is not “we are already faster than NASA / TBM industry / SOTA.” It is that a common, provenance-preserving protocol can compare material response, solver fidelity, blind generalization and end-to-end delivered service without hiding losses or uncertainty.

## CI fixture guard

The Linux CI step checks the synthetic schemas, all three partition records, unique run IDs, complete run/manifest membership, all four core plus optional lab/campaign axes, blind-test freeze flags, measurement status/provenance, prediction-interval ordering, artifact reference resolution, actual artifact SHA-256 values, and force-trace peak/mean/impulse/work/displaced-volume arithmetic. It runs the reusable `scripts/validate-lunar-excavation-dataset.sh` against the full synthetic dataset.

For a real dataset, invoke the same CLI from the repository root:

```bash
bash scripts/validate-lunar-excavation-dataset.sh \\
  path/to/dataset-manifest.json \\
  path/to/runs/*.json
```

Artifact paths inside each run record are resolved relative to that run file's directory. The validator requires the supplied run files to match all manifest assignments exactly, and by default rejects placeholder paths. The `--allow-synthetic-placeholders` option is reserved for the committed synthetic CI fixtures. The tool performs semantic and provenance checks; it is not a complete JSON Schema implementation, an independent blind-data custodian, or proof that the measurement values are physically true.


## New published comparators added to the tournament — 18 November 2026

A further 2026 machine-learning result must now be treated as a named baseline:

- **Srinivasan et al., Acta Astronautica (available online 18 August 2026), DOI 10.1016/j.actaastro.2026.08.051:** compares multiple linear regression, SVR, random forest and XGBoost for horizontal and vertical lunar-regolith excavation forces. XGBoost reports RMSE 11.305 N horizontal and 6.824 N vertical, and a 64% / 85.5% reduction versus the Lockheed–Martin analytical model. This is a published result on its study dataset, not demonstrated out-of-distribution transfer. Reproduce its precise feature set, split, denominators and test protocol before treating those RMSE values as a matched target.
- **Bao et al., Acta Astronautica (available online 3 November 2026), DOI 10.1016/j.actaastro.2026.10.071:** tests authentic Chang'e-6 farside sample CE60100 in a rotating drum in the slumping regime. The available sample mass was only 1.85 g. Dynamic angle of repose decreased from 50.4° ± 3.1° at Fr = 3.3×10^-4 to 41.1° ± 1.8° at Fr = 3.3×10^-3; the model reproduced the Fr trend without fitting the reported CE-6 DAoR values. This is a valuable flow/handling reference, not an excavation-force validation set or broad lunar-material sample.
- **NASA's current Lunar Surface Technology page:** describes IPEx as designed for 10 metric tonnes over 100 m and 11 days; the same public programme material also describes an approximately 20-tonne single-mission goal. Do not silently merge the two figures into one achieved productivity result; preserve mission profile, test boundary and goal/status label. NASA's 2026 ISAM State of Play separates excavation/site preparation, regolith processing, logistics and construction—useful evidence that the field's challenge is integrated surface infrastructure, not a single excavator metric.

### Expanded model tournament

The matched test set should compare, where the input data and permissions allow:

1. Lockheed–Martin analytical baseline and other declared analytical/empirical equations.
2. Published MLR, SVR, random-forest and XGBoost models, including the 2026 Srinivasan et al. XGBR baseline.
3. DEM with the source-matched calibration strategy (including Sun et al.'s JLU-H result).
4. CEL/FEM, including static and percussive JSC-1A cases.
5. A physics-informed residual model: preserve physically meaningful force/energy constraints and allow a learned residual only inside the declared evidence envelope. Compare it against pure XGBR and each physics solver on identical held-out groups; no extrapolation is silently accepted.
6. For material flow, a separate rotating-drum / hopper / transfer benchmark comparing force-balance, DEM and measured flow regimes. Keep flow-model scorecards separate from excavation-force scorecards.

The ML tournament must publish two different claims:

- **Interpolation:** held-out trials from already represented batches and operating envelopes.
- **Transfer:** entirely unseen material batches, compaction histories, tool geometries, laboratories, gravity levels or operation modes.

Do not call a random row split a transfer test. Include leave-one-batch-out and leave-one-laboratory-out partitions where the source corpus supports them. Keep a blind external set controlled by an independent custodian. Report calibration error, interval coverage, worst-group error, out-of-domain detection and abstention alongside RMSE. A model that recognizes it is outside its envelope and declines to predict can be safer and more useful than a lower-RMSE model that confidently extrapolates.

### New physical-flow track

Treat the flow of excavated material as its own benchmark:

- authentic returned lunar material where access, mass limits and permissions permit;
- terrestrial simulants each remain distinct by batch, size distribution, morphology and preparation;
- rotating drum, hopper discharge, transfer chute and separator are separate boundary conditions;
- report dynamic angle of repose, avalanche statistics, mass discharge rate, residual/retained mass, bridging/jam rate, transfer efficiency and energy per accepted delivered mass;
- test gravity and Froude-number effects without assuming that Earth-g measurements directly scale to lunar gravity.

The Chang'e-6 study is unusually useful but its 1.85-g retained sample and slumping regime are strict scope limits. It should seed a targeted micro-sample flow-reproduction protocol, not a general regolith handling calibration.

## Authority boundaries

- Symtropy owns immutable inputs, schemas, run orchestration, replay and evidence receipts.
- Specialist DEM/CEL/FEM solvers own numerical and physical model calculations.
- Symthaea may select the next informative experiment and surface disagreements; it cannot certify the data, tune away blind residuals, or accept its own recommendations as independent evidence.
- Mycelix can preserve provenance, identity and custody; it cannot make an inaccurate sensor accurate.
- Independent physical test operators and reviewers own instrument calibration, test execution and acceptance.

## Claim ceiling

The example is synthetic. A valid JSON record is not a valid physical experiment, a valid experiment is not a qualified model, and a qualified simulant model is not lunar soil, competent-rock tunnel, machine-productivity or flight qualification.
