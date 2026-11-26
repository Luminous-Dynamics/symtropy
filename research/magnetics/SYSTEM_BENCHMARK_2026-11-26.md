# Magnetic systems: industry comparison and benchmark contract

**Date:** 26 November 2026  
**Status:** proposed benchmark gates; none of the target values below are achieved results.

## 1. The market is several different problems

Do not put magnetic field solvers, Hall latches, magnetic absolute encoders and optical metrology on one accuracy leaderboard.

| Comparator | Published capability | What it is a fair comparator for | What it is not |
|---|---|---|---|
| TI TMAG5110/TMAG5111 | 2D dual-channel Hall latch; typical BOP variants 1.4 mT and 3 mT; product page lists 40 kHz bandwidth and -40°C to +125°C; independent A/B latch output or speed/direction output | Hall field-to-digital transition chain, speed/direction, packaging, field margin and operating-temperature challenge | A calibrated absolute-angle system accuracy specification |
| RLS AksIM-2 | Axial off-axis magnetic absolute encoder; 17–20-bit resolution, up to 10,000 rpm (size-dependent); system accuracy ±0.004° to ±0.020° after self-calibration in the May 2026 data sheet; hysteresis and repeatability below one unit of resolution | High-end magnetic absolute encoder system, including calibration and ring/readhead integration | A raw Hall-latch sensor or a solver benchmark |
| Renishaw RESOLUTE | Optical absolute rotary system; published typical installed accuracy varies with ring size, approximately ±12.7 arcsec for 52 mm down to ±3.6 arcsec for 255 mm in the cited configuration | Upper industrial optical metrology reference and independent rotor-angle truth source | A direct magnetic BOM/environment competitor without application normalization |
| Magpylib Material Response 0.15.0 | Analytical-source and face-charge browser/WASM kernels; face-charge differentiable through JAX/PyTorch; release source describes shared 50-digit references | Open solver and differentiable-kernel reference for value/gradient qualification | A complete replacement for nonlinear/an-isotropic FEM or physical metrology |

Sources:
- TI TMAG5110/TMAG5111 product page: https://www.ti.com/product/TMAG5111
- TI TMAG511x datasheet Rev D: https://www.ti.com/lit/ds/symlink/tmag5111.pdf
- RLS AksIM-2 data sheet MBD01_15, issue 15, 13 May 2026: https://www.rls.si/cn_tw/fileuploader/download/download/?d=0&id=267&title=Data+sheet%3A+AksIM-2%E2%84%A2+off-axis+rotary+absolute+magnetic+encoder+%28MBD01%29
- RLS AksIM-2 official product page: https://www.rls.si/eng/aksim-2-off-axis-rotary-absolute-encoder
- Renishaw RESOLUTE technical system data: https://www.renishaw.com/resourcecentre/download/%288a82e17fae1b46118c029a0c0f9594f4%29?userLanguage=en
- Magpylib Material Response 0.15.0 PyPI release and artifact digests: https://pypi.org/project/magpylib-material-response/0.15.0/
- Magpylib Material Response 0.15.0 source commit: https://github.com/magpylib/magpylib-material-response/commit/492b37459a47f4220d7e44380668c2d9ab720784

Important qualification note: TI product-page operating-point summaries include typical and maximum values for different device variants; the Rev D datasheet's threshold curves are typical characteristics. Do not turn a typical curve or graph digitization into a guaranteed tolerance interval. Obtain the exact part suffix, guaranteed electrical table, temperature/supply conditions and traceable component characterization before creating a hard sensor bound.

## 2. Paradigm shift: optimize the physical measurement function

The design variable is not just the magnet. It is the coupled system:

    magnetization pattern + material + geometry + air gap + sensor axes
      + sensor thresholds + calibration + decode + operating envelope

The objective is a verified functional margin over the full declared envelope, not maximum field strength, ideal quadrature at one nominal point, or best RMSE on a calibration map.

Three coupled improvements matter:

1. **Response-aware magnetic design:** choose pole pattern, magnetization depth and geometry against downstream response margins (transition phase, edge spacing, missed-edge reserve, external-field immunity), instead of optimizing a generic field norm.
2. **Distribution-free sensor robustness:** propagate traceable simultaneous field, axis, BOP/BRP, air-gap and geometric bounds; do not manufacture probability from a handful of scenarios or typical datasheet curves.
3. **Decision-directed metrology:** measure where qualified model intervals are separated and where the declared functional margin is sensitive, keeping blind holdout points physically and logically separate.

## 3. Scoreboards must remain separate

### A. Electromagnetic field model

Report vector-field errors, not only a scalar magnitude:
- RMS and maximum absolute vector error in tesla;
- normalized vector error using a declared field floor, so near-zero reference points do not explode or disappear;
- component-wise Bx/By/Bz, direction error, local gradient/Jacobian error;
- boundary/near-surface, near-axis, far-field and ill-conditioned strata;
- convergence estimate, actual error against an independent reference, runtime and peak memory;
- solver applicability and excluded physical regimes.

Suggested internal qualification gates (aspirational, not literature SOTA claims):
- analytic known-answer cases: relative vector error ≤ 1e-6 away from singular/ill-conditioned points;
- matched coupled-material held-out cases: ≤ 0.25% RMS and ≤ 1% maximum normalized vector error, with no unreported outlier removal;
- gradient checks on smooth, well-conditioned strata: ≤ 1% normalized directional-derivative error against independent high-precision references;
- empirical convergence-estimator coverage: report the fraction of a preregistered validation suite in which the declared error bound encloses the observed reference error; target ≥ 95%, while reporting every miss.

Do not apply these thresholds to a different model class merely because both solvers return B. Face-charge for linear isotropic susceptibility, cell methods for their supported material model, and FEM must state applicability separately.

### B. Numerical solver and differentiable inverse design

Pin artifact hashes, source commit, dependency versions, runtime, precision, backend, inputs and reference data. For the 0.15.0 wheel, PyPI reports SHA-256 `31ba18ce68114fc67627f4baa83d9a578041b79afae62632857b35bf818eda2f`; the source distribution is `763c6d111abf59a0823ff35b83e31b7a7a8d24ca5a46239ab9374e14a807ab99`.

Compare field values and derivatives independently. Check rotational/translation covariance, scale laws, source superposition, mesh/panel refinement, finite-difference step stability, near-axis/far-field behaviour and degenerate geometry. JAX eager/JIT, PyTorch eager, WASM and high-precision reference runs need matched numerical tolerances before speed comparisons.

Inverse design should use differentiable fast models for exploration, then re-check finalists with independent FEM/refinement and calibrated physical measurements. A differentiable solver is an optimizer aid, not proof that the physical design is correct.

### C. Incremental Hall quadrature

Fair baseline: same ring geometry, pole count, field amplitude, sensor placement, BOP/BRP conditions, RPM, acquisition reference and sampling grid.

Publish:
- A/B transition counts and Gray-code legality;
- missing/extra edges, reversals and direction errors;
- mechanical/electrical edge-spacing error with interval bounds;
- worst-case phase error across air gap, eccentricity, tilt, temperature, speed and threshold tolerances;
- threshold/field reserve in tesla and normalized to the exact guaranteed threshold interval;
- latency, edge jitter, maximum event rate, dropped-edge count and power;
- physical-capture agreement against an independent reference encoder.

Proposed internal gates:
- zero illegal two-bit transitions and zero missing/excess edges over every deterministic bounded challenge case;
- ≤ 1.0° mechanical worst-case quadrature spacing error on the TI SBAA449B reproduction challenge, improving on its cited 4.9° discrete-latch mismatch analysis by at least 4.9×;
- ≤ 0.5° on the separately modelled integrated 2D Hall challenge, versus the approximately 1.35° TI analysis result;
- no dropped/extra events at the declared maximum speed and at 2× the target event rate, with measured latency/jitter and no substitution of sensor bandwidth for encoder output-rate capability.

Those numbers are proposed engineering targets. They are not measured performance and the simplified sinusoidal TI reconstruction is not a full reproduction of the original 3D field model.

### D. Magnetic absolute encoder system

Only if the programme elects to build an absolute-angle product, benchmark against RLS AksIM-2 with the same ring size, installation, calibration and thermal protocol. An ambitious first system target is ≤ ±0.020° after calibration over the declared envelope; a stretch target is ≤ ±0.010°. The aspirational gate is deliberately not claimed to beat the current AksIM-2 upper-performing variants at ±0.004°.

Report resolution, accuracy, repeatability, hysteresis, installed eccentricity/runout sensitivity, temperature drift, speed, bandwidth, external-field susceptibility, calibration burden, power, BOM and serviceability separately. Compare optical RESOLUTE as a metrology reference, not as a direct like-for-like magnetic product.

### E. Active metrology

Compare uniform grid, seeded random, field-variance, cross-solver disagreement, sensitivity-only, decision-directed lower-bound planner and a full-map oracle. Freeze the blind holdout before adaptive sampling.

Report measurements and acquisition seconds to stabilize the pass/fail decision, worst blind-holdout functional error, false accepts/rejects, missed risk regions, probe re-registration and calibration overhead. Initial target: at least 2× fewer adaptive observations than the uniform-grid baseline at the same blind-holdout decision error, with zero blind-data leakage. This is an empirical benchmark target, not an assumed benefit.

## 4. Qualification ladder

| Tier | Evidence | Advancement criterion |
|---|---|---|
| T0 — arithmetic | Analytic fields, exact geometry, synthetic traces, property/metamorphic tests | Known-answer and invariance controls pass |
| T1 — independent numerics | Matched Magpylib, face-charge/cell and FEM runs with refinement | Applicability, convergence and independent cross-check recorded |
| T2 — blind challenge | Frozen geometries, material conditions and observer grids not used for tuning | Every result carries exact input/code/reference identities; report every miss |
| T3 — calibrated bench | Physical ring, characterized Hall device, calibrated rotor reference and probe calibration | Measured uncertainty and synchronization budget closes; independent capture reconciles |
| T4 — production envelope | Multiple lots, tolerance extremes, thermal/speed/environment cycles | Statistical claims only with a preregistered sampling plan and representative manufacturing data |

Do not promote a T0 synthetic pass to a T3 physical qualification. Do not call queued, skipped or cancelled CI steps passing.

## 5. Where we can genuinely outperform

The credible differentiation is the end-to-end evidence chain:

    geometry/material proposal -> qualified field model -> robust response model
      -> candidate ranking -> decision-directed physical measurement
      -> independent capture -> blind functional acceptance

Commercial products may already win on accuracy, certification, installed base or manufacturing scale. Our opportunity is to demonstrate a better *design-to-evidence workflow*: explicit applicability, reproducible multi-solver checks, robust functional margins, fewer necessary measurements and transparent physical reconciliation. It becomes a competitive claim only after the matched T2/T3 benchmark suite is complete.

## 6. Active work

- Hall quadrature and bounded sensor tolerances: [PR #7449](https://github.com/Luminous-Dynamics/symtropy/pull/7449), [issues #7531](https://github.com/Luminous-Dynamics/symtropy/issues/7531) and [#7538](https://github.com/Luminous-Dynamics/symtropy/issues/7538).
- Solver evidence contract and 0.15.0 derivative qualification: [PR #7445](https://github.com/Luminous-Dynamics/symtropy/pull/7445), [issue #7536](https://github.com/Luminous-Dynamics/symtropy/issues/7536).
- Active metrology: [PR #7540](https://github.com/Luminous-Dynamics/symtropy/pull/7540), [issue #7539](https://github.com/Luminous-Dynamics/symtropy/issues/7539).
- Continuous vector-phase encoder: [PR #7413](https://github.com/Luminous-Dynamics/symtropy/pull/7413).
- Bounded phase assurance: [PR #7475](https://github.com/Luminous-Dynamics/symtropy/pull/7475).

## Claim boundary

This file sets proposed benchmark protocols and target gates. It does not report benchmark completion, physical measurement, commercial superiority, global design optimality or certification.