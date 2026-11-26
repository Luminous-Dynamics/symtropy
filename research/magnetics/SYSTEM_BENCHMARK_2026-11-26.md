# Magnetic systems: industry comparison and benchmark contract

**Date:** 26 November 2026  
**Status:** proposed benchmark gates; none of the target values below are achieved results.

## 1. The market is several different problems

Do not put magnetic field solvers, Hall latches, magnetic absolute encoders and optical metrology on one accuracy leaderboard.

| Comparator | Published capability | What it is a fair comparator for | What it is not |
|---|---|---|---|
| TI TMAG5110/TMAG5111 | 2D dual-channel Hall latch; typical BOP variants 1.4 mT and 3 mT; product page lists 40 kHz bandwidth and -40°C to +125°C; independent A/B latch output or speed/direction output | Hall field-to-digital transition chain, speed/direction, packaging, field margin and operating-temperature challenge | A calibrated absolute-angle system accuracy specification |
| RLS AksIM-2 | Axial off-axis magnetic absolute encoder; up to 20-bit; 10,000 rpm; official page lists system accuracy ±0.004° to ±0.020° after self-calibration (configuration-dependent) | High-end magnetic absolute encoder accuracy, calibration, ring/readhead integration | A Hall latch or a solver benchmark |
| RLS AksIM-4 (standard family) | Off-axis true-absolute magnetic encoder; up to 22-bit; ring outer diameter 22–150 mm; connector operating/storage range -40°C to +105°C or solder-pad range to +125°C | General-purpose high-resolution magnetic absolute encoder, package and temperature envelope | HDSL-specific accuracy values, which must not be generalized to every ring/readhead variant |
| RLS AksIM-4 HIPERFACE DSL (MCD06) | 20/22-bit variants; 53/80 mm rings; system accuracy ±0.006° (MC053) / ±0.004° (MC080); readhead -40°C to +115°C and ring -40°C to +125°C; current product page links MCD06 | Most relevant current RLS HDSL magnetic absolute comparator; exact variant, ring and calibration must match | An installed accuracy result or a generic accuracy guarantee for every AksIM-4 |
| RLS AksIM-4 Dual Concentric | Two absolute channels in one encoder; 19-bit input and output; stated system accuracy up to ±0.010°; -40°C to +105°C | Dual-sensor robot-joint feedback and compact dual-measurement architectures | A single-channel Hall latch |
| Renishaw RESOLUTE ETR (RESA30) | Optical absolute rotary system; current ETR datasheet reports typical installed accuracy ±12.7 arcsec at 52 mm, ±4.3 arcsec at 200 mm and ±1.9 arcsec at 550 mm; corresponding maximum reading speeds 18,000, 4,750 and 1,700 rpm. The datasheet explicitly notes 32-bit resolution below the encoder noise floor. | Optical installed-angle/metrology reference and independent rotor-angle truth source | A direct magnetic BOM/environment competitor without application normalization |
| Magpylib Material Response 0.15.0 | 25 Nov 2026 release adds analytical and face-charge WASM field kernels checked against shared 50-digit references; face-charge runs through JAX/PyTorch with JIT and gradients; changelog reports a cylinder far-axis stability regression at 576 radii where the previous formula could lose up to 29% of digits | Open field-kernel, differentiable face-charge and numerical-stability reference | A replacement for nonlinear/anisotropic FEM or physical metrology |

Sources:
- TI TMAG5110/TMAG5111 product page: https://www.ti.com/product/TMAG5111
- TI TMAG511x datasheet Rev D: https://www.ti.com/lit/ds/symlink/tmag5111.pdf
- RLS AksIM-2 official product page: https://www.rls.si/eng/aksim-2-off-axis-rotary-absolute-encoder
- RLS AksIM-4 standard family official product page: https://www.rls.si/eng/aksim-4-off-axis-rotary-absolute-encoder
- RLS AksIM-4 HIPERFACE DSL official product page (current page links MCD06): https://www.rls.si/eng/aksim-hdsl-rotary-absolute-magnetic-encoder
- RLS current absolute-encoder portfolio (standard family and HDSL variant distinction): https://www.rls.si/eng/products/rotary-magnetic-encoders/absolute-encoders
- RLS AksIM-4 Dual Concentric official product page: https://www.rls.si/eng/aksim-dual
- RLS MCD05 Issue 04 lifecycle notice; directs new designs to MCD06: https://cdn.rls.si/documents/datasheets/MCD05.pdf
- Renishaw RESOLUTE ETR datasheet, installed accuracy by RESA30 ring diameter: https://www.renishaw.com/resourcecentre/download/%28844aab74daa246dd80cff7ec2e451af0%29?userLanguage=en
- Magpylib Material Response 0.15.0 changelog: https://magpylib-material-response.readthedocs.io/en/stable/changelog.html
- Magpylib Material Response 0.15.0 PyPI release and artifact digests: https://pypi.org/project/magpylib-material-response/0.15.0/
- Magpylib Material Response 0.15.0 source commit: https://github.com/magpylib/magpylib-material-response/commit/492b37459a47f4220d7e44380668c2d9ab720784
- Magpylib Material Response 0.15.0 changelog (576-radius cylinder stress case): https://magpylib-material-response.readthedocs.io/en/stable/changelog.html
- Insinga and Bjørk, “Gradient-based optimization of permanent-magnet assemblies for any objective”, Physical Review Applied 20, 064030 (2023): https://doi.org/10.1103/PhysRevApplied.20.064030

Important qualification note: TI product-page operating-point summaries include typical and maximum values for different device variants; the Rev D datasheet's threshold curves are typical characteristics. Do not turn a typical curve or graph digitization into a guaranteed tolerance interval. Obtain the exact part suffix, guaranteed electrical table, temperature/supply conditions and traceable component characterization before creating a hard sensor bound.

For RLS, distinguish the AksIM-4 standard family page from the HDSL MCD06 variant page. The standard family page advertises up to 22-bit and 22–150 mm rings but does not license applying the HDSL MC053/MC080 system-accuracy numbers to all standard AksIM-4 configurations. The MCD05 page now carries a new-design warning and points users to MCD06. For Renishaw RESOLUTE ETR, the cited values above are *typical installed accuracy* by ring diameter, not an interchangeable system-accuracy specification. Match ring size, installation, readhead count, calibration and speed before comparing any of these values.

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

Add a pinned cylinder-axis stress case at 576 cylinder radii. Magpylib Material Response 0.15.0's changelog reports that the previous formula could lose up to 29% of digits at that distance and that the new kernel preserves high-precision reference digits. Reproduce with fixed cylinder geometry, observer coordinate and independent high-precision reference; compare 0.14.x and 0.15.0 artifact hashes, normalized vector error and derivative behaviour. This is an upstream-reported regression target, not a Symtropy result until independently reproduced.

Inverse design should use differentiable fast models for exploration, then re-check finalists with independent FEM/refinement and calibrated physical measurements. A differentiable solver is an optimizer aid, not proof that the physical design is correct.

### F. Function-first inverse design

Gradient-based permanent-magnet design is established research, not a novelty claim. Insinga and Bjørk demonstrated a semianalytical adjoint-like method for optimizing magnetization direction over a prescribed design region. Our opportunity is to optimize the *encoder's measured function and its evidence margin*, then qualify each finalist independently.

Compare these optimizers under identical parameterizations, initial designs, material/geometry constraints, compute budget and preregistered holdout scenarios:

- engineering baseline (existing pole pattern and hand tuning);
- bounded grid/random search;
- derivative-free CMA-ES or another fixed evolutionary baseline;
- Bayesian optimization with an explicit evaluation budget;
- gradient/adjoint search through a differentiable field model;
- multi-fidelity search: differentiable/fast model for exploration, independent FEM for finalist screening, and bounded physical metrology for final acceptance.

The design vector must declare which variables are actually free: magnetization direction, pole pitch and duty, magnet thickness/segmentation, material, air gap, eccentricity allowance, sensor axes/position and calibration degrees of freedom. Do not silently optimize sensor parameters that the product cannot calibrate or control.

The objective is constrained and multiobjective, not a hidden weighted scalar. Candidate feasibility is decided first by hard worst-case functional limits (phase/edge reserve, minimum field, ripple, cross-axis leakage and environment). Only feasible candidates enter a Pareto ranking for robust margin, BOM/material use, package envelope, compute and measurement cost.

Proposed research targets, not achieved results:
- at least 2× fewer high-fidelity FEM evaluations than a single-fidelity baseline to reach the same preregistered robust-feasible hypervolume under equal total compute;
- every finalist passes derivative directional checks against independent finite differences and an independent solver within declared tolerances;
- zero false accepts on the deterministic bounded challenge set;
- no candidate may be accepted using training scenarios alone: hidden geometric, thermal, eccentricity and stray-field cases remain sealed until the final evaluation;
- publish time-to-first-robust-feasible design, number of FEM solves, wall-clock/energy, Pareto hypervolume, failure rate, and worst blind functional margin.

The 2× target is a falsifiable research target, not a promised gain. Report solver calls and hardware as well as elapsed time, because different fidelity levels have different costs.

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

Only if the programme elects to build an absolute-angle product, use the current RLS AksIM-4 HIPERFACE DSL MCD06 variants as the primary magnetic absolute challenge: MC053 (53 mm ring, published system accuracy ±0.006°) and MC080 (80 mm ring, published system accuracy ±0.004°). Match the exact ring, installation, readhead count, self-calibration procedure and thermal protocol. A first parity target is ≤ ±0.004° system accuracy on the MC080-matched challenge; the stretch target is ≤ ±0.003°. For the MC053-sized build, target ≤ ±0.005° against its published ±0.006° system figure. These are research targets, not achieved results. System accuracy and typical/actual installed accuracy are different observables and must be measured/reported separately; do not silently treat the vendor system figure as an installed-field result.

Report resolution, system accuracy, calibrated and uncalibrated installed accuracy, repeatability, hysteresis, installed eccentricity/runout sensitivity, temperature drift, speed, bandwidth, external-field susceptibility, calibration burden, power, BOM and serviceability separately. Compare optical RESOLUTE as a metrology reference, not as a direct like-for-like magnetic product.

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

A specific claim boundary follows from the current market: AksIM-4's high-resolution options reach 22 bits, but this is distinct from its size-dependent system accuracy (±0.020° to ±0.004°); its typical installed calibrated accuracy is looser (±0.030° to ±0.009°), and uncalibrated accuracy is looser again (±0.100° to ±0.029°). The data sheet states that system accuracy can be achieved with two readheads 180° apart or under optimal self-calibration conditions. AksIM-2 also publishes ±0.004° to ±0.020° system accuracy by configuration. RESOLUTE remains an optical metrology class with tighter ring-dependent and two-readhead installed accuracy. Compare installed-to-installed, ring-size-to-ring-size and calibration-to-calibration; never compare the AksIM-4 maximum bit depth with optical accuracy.

## 6. Active work

- Hall quadrature and bounded sensor tolerances: [PR #7449](https://github.com/Luminous-Dynamics/symtropy/pull/7449), [issues #7531](https://github.com/Luminous-Dynamics/symtropy/issues/7531) and [#7538](https://github.com/Luminous-Dynamics/symtropy/issues/7538).
- Solver evidence contract and 0.15.0 derivative qualification: [PR #7445](https://github.com/Luminous-Dynamics/symtropy/pull/7445), [issue #7536](https://github.com/Luminous-Dynamics/symtropy/issues/7536).
- Active metrology: [PR #7540](https://github.com/Luminous-Dynamics/symtropy/pull/7540), [issue #7539](https://github.com/Luminous-Dynamics/symtropy/issues/7539).
- Continuous vector-phase encoder: [PR #7413](https://github.com/Luminous-Dynamics/symtropy/pull/7413).
- Bounded phase assurance: [PR #7475](https://github.com/Luminous-Dynamics/symtropy/pull/7475).

## Claim boundary

This file sets proposed benchmark protocols and target gates. It does not report benchmark completion, physical measurement, commercial superiority, global design optimality or certification.

## 7. Validated sensitivity and global-search research (26 November 2026)

The new software research layers are separate and conditional:
- [#7536](https://github.com/Luminous-Dynamics/symtropy/issues/7536) / [#7635](https://github.com/Luminous-Dynamics/symtropy/pull/7635): qualify directional gradients against independent references and central-difference plateaus.
- [#7653](https://github.com/Luminous-Dynamics/symtropy/issues/7653) / [#7655](https://github.com/Luminous-Dynamics/symtropy/pull/7655): interval forward-mode AD, currently limited to algebraic/rational expressions and square root, producing cell-wide derivative enclosures under a declared binary64 outward-expansion model.
- [#7639](https://github.com/Luminous-Dynamics/symtropy/issues/7639) / [#7642](https://github.com/Luminous-Dynamics/symtropy/pull/7642): continuous-domain functional-margin certification, conditional on valid component-wise derivative supremum bounds over each cell.
- [#7658](https://github.com/Luminous-Dynamics/symtropy/issues/7658) / [#7660](https://github.com/Luminous-Dynamics/symtropy/pull/7660): interval branch-and-bound for a single scalar objective and robust margin. v0.1 keeps one unsplit nuisance box and is stacked on interval AD.

These are research implementations, not verified product capabilities or SOTA results. The global-search branch is intentionally stacked; current CI filters PR runs to `main`, so #7660 does not yet receive a standalone PR-triggered matrix run. Do not report absent checks as queued or passed.

