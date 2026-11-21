# PWR-TX-HST-001 — Thermal surrogate and lifetime decision benchmark

**Status:** research protocol proposal; no transformer thermal result is claimed by this document.

## Why this is the next frontier

The existing benchmark screens core loss and audits surrogate metrics, split integrity, annual traces, and evidence-file hashes. That is useful, but a lifecycle claim requires more than core-loss prediction. Losses feed the thermal state; hotspot temperature feeds insulation ageing; ageing feeds lifecycle cost and replacement/loading decisions. A small core-loss error can still produce a bad lifecycle decision when load/ambient trajectories or thermal limits are near a constraint.

## Relevant state of the art and the fair comparison

Achour et al., *Remaining useful life of power transformers using efficient surrogates and deep learning techniques* (published 13 June 2026), build a sparse Proper Generalized Decomposition (sPGD) surrogate around a three-dimensional CFD thermal model. The article reports a relative L2 error of 1.19% over 50 CFD-generated samples split 80/20 and less than two seconds to simulate a 50-year hotspot trajectory. Its degradation experiments use 50 samples with a 90/10 split. The paper also evaluates controlled measurement-noise sensitivity; at 10 dB SNR the long-horizon trajectory diverges substantially. Source: https://doi.org/10.1186/s40323-026-00329-6.

These values are a reference for a reproduction attempt, not proof of generalization across transformer geometries or evidence of field-calibrated RUL. The paper's 50-year case uses ambient data from the Paris area (1970–2020) and estimates load from ambient temperature using stated assumptions; the authors note actual load traces can be used when available. That distinction must be preserved in any comparison: reproducing a model-generated aging trajectory is not the same as validating time-to-failure on a representative cohort of operating transformers.

The normative engineering baseline for mineral-oil-immersed transformers is the applicable edition of IEC 60076-7; the IEC catalogue lists IEC 60076-7:2018 as valid and describes its scope as the effect of ambient temperature and loading on operating temperatures and thermal ageing: https://webstore.iec.ch/en/publication/34351. Applicability, national adoption, amendments and exact project requirements still need qualified review. This protocol must not treat a thermal surrogate as a compliance determination.

## Benchmark ladder

1. **Analytical baseline:** independently implement a pinned and reviewed IEC 60076-7:2018 reference case for the declared mineral-oil-immersed scope. Record all coefficients, initial conditions, load/ambient series, units, numerical time step and interpretation.
2. **High-fidelity baseline:** use a reproducible 3D conjugate heat transfer (CHT) reference with declared solver/version, mesh and time-step convergence checks, energy-balance residuals, boundary conditions and material properties. Retain the input deck, mesh and convergence receipts.
3. **Reduced-order baselines:** compare an equivalent thermal network and at least one established reduced-order method (sPGD or POD/ROM); include SINDy/Neural ODE only for a defined dynamical-learning task. A Symtropy proposal is a candidate under test, not a privileged oracle.
4. **Independent validation:** where data are available, compare CFD/ROM output with measured temperature-rise tests and measured hotspot data under recorded loading. Separate simulation agreement from physical validity.
5. **Lifecycle decision trial:** use each model in the same loading/derating/replacement optimization. Measure decision regret and unsafe recommendations, not only temperature regression error.

## Splits that are mandatory for transfer claims

The split ledger must be frozen before model selection and checked by `split-audit`. Report metrics independently for:

- random-row split (diagnostic baseline only);
- held-out geometry family;
- held-out manufacturer or transformer design family;
- held-out thermal/measurement campaign;
- held-out operating trajectories and ambient-climate profiles;
- unseen load ramps, overload durations and ambient/load combinations within the explicitly declared model scope.

When the dataset is too small to support a group-held-out test, state that limitation instead of substituting a random split and claiming generalization. The manifest should bind dataset, preprocessing, split ledger, solver input/output, surrogate artifact, evaluation protocol and query/runtime ledger using `evidence-receipt`.

## Metrics: field error is not enough

### Thermal trace and transient behaviour

- MAE, RMSE, 95th-percentile absolute error and maximum absolute error for hotspot and top-oil/winding temperatures where reference data exist; state errors in °C and normalized form.
- Peak hotspot error, time-to-peak error, heating/cooling time-constant error, ramp response and overshoot.
- Energy/thermal-balance residual and physical-consistency checks.
- Prediction-interval coverage and interval width by held-out stratum.

### Ageing and lifecycle decisions

- Error in cumulative equivalent ageing/loss-of-life integral relative to the declared reference model.
- Failure-horizon/RUL error only where the target's construction is explicit; separate generated-reference RUL from field-observed failure outcomes and account for censored survivors.
- Decision regret for selected loading, derating, replacement and design decisions against the reference evaluator.
- False-safe rate: fraction of cases where the surrogate recommends operation that the qualified reference indicates violates the declared thermal constraint.
- Out-of-domain detection/abstention performance.

### Total cost

- End-to-end wall-clock and memory on pinned hardware.
- Number and cost of all high-fidelity runs, training, tuning, failed/retried jobs, calibration, mesh refinements and final verification.
- Amortized cost per qualified decision over realistic reuse counts, not just per-inference latency.
- Report performance curves at matched expensive-simulation budgets; include an equivalent thermal network, the IEC analytical baseline, a conventional ROM, and a simple sweep/optimizer.

## Proposed research targets

These are aspirational gates for comparable, declared test domains, **not standards or achieved Symtropy results**:

| Gate | Initial target | Qualification caveat |
|---|---|---|
| Published sPGD reproduction | Match the paper's stated relative L2 metric on compatible inputs/splits, then report any discrepancy | Do not compare percentages unless output, normalization and split are identical |
| Held-out hotspot trajectory | MAE ≤2 °C; P95 absolute error ≤5 °C on the pre-registered held-out panel | Use sensor/reference uncertainty and project risk limits to set final acceptance |
| Transient response | Peak timing error ≤5% of the reference time-to-peak; no hidden unstable/oscillatory response | Must include step/ramp and load-recovery cases |
| Ageing integral | ≤5% relative error against the specified ageing reference on in-domain cases | This is not yet field RUL validation |
| Uncertainty calibration | 90% and 95% intervals reported, with empirical coverage by stratum within ±3 percentage points of nominal where sample size supports it | Publish sample counts and uncertainty on coverage estimates |
| Thermal safety challenge set | Zero false-safe recommendations in a frozen, finite challenge panel; any false-safe fails that panel | Finite tests do not prove universal safety |
| Runtime/cost | Target <2 s for the 50-year scalar trajectory on explicitly pinned hardware when reproducing the published setting; additionally report total amortized compute | The paper's runtime is hardware/method-specific and must not be generalized without matched conditions |
| Expensive-simulation reduction | ≥90% lower total high-fidelity cost at matched decision quality as an initial gate; aim higher only if measured | Count training, failed runs, refinement and final verification |
| OOD handling | Unsupported geometry/material/cooling/ambient domain causes abstention or explicit escalation | Silent extrapolation is a failed gate |

Targets should be tightened or relaxed only by a documented risk-based rationale and reference uncertainty—not by tuning after viewing the test set.

## Global transfer contract

The canonical physical inputs should include transformer technology/cooling class, geometry/winding revision, ratings, material/insulation properties and their source, loss map/reference temperature, ambient and load time series, initial thermal state, boundary conditions, and all solver/model revisions. UTC trace integrity is necessary but does not validate sensor calibration or thermal representativeness. Store the named IANA timezone and local interpretation for provenance; validate any local-to-UTC/DST conversion upstream.

Regional profiles should express applicable loading/thermal limits and utility requirements separately. Do not generalize an IEC mineral-oil loading guide to dry-type equipment without a separate justified model and standard pathway. Unknown inputs remain unknown rather than silently receiving a default.

## Stop conditions and evidence levels

- A lower average temperature error does not compensate for increased false-safe rate.
- A solver-relative surrogate result must not be described as physical validation.
- A generated 50-year trajectory does not validate a 50-year failure prediction.
- No design or operating recommendation can be called qualified without independent review against applicable project constraints.
- All benchmark receipts must bind the exact inputs, source commit, model/solver settings, split, results and evaluator used for that claim.

## Suggested implementation sequence

1. Reproduce a small, published analytical/thermal reference case with a hand-checkable oracle.
2. Create a pinned small 3D CFD benchmark fixture with mesh/time convergence and an evidence manifest before training any model.
3. Implement the reference thermal-network baseline and transient challenge tests.
4. Add a ROM comparison and held-out split results; calibrate uncertainty and abstention.
5. Only then evaluate lifecycle optimization, load management and design recommendations.
