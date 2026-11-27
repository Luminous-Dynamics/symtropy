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
| RLS AksIM-2 (MBD01 Issue 15, 13 May 2026) | Off-axis magnetic; MRA080 (80 mm OD) up to 20-bit; system accuracy ±0.004° (14.4 arcsec); typical installed accuracy ±0.009° calibrated / ±0.029° uncalibrated; max speed 4,700 rpm at size 080; 9 kHz bandwidth, 18 kHz sampling, up to 54 kHz refresh; -40°C to +105°C; manufacturer lists no thermal drift. Footnote defines system accuracy as installed accuracy with two readheads or best possible after self-calibration. | Primary size-matched magnetic benchmark for an 80 mm design; use the 80 mm installed accuracy rows as the direct calibration/install comparison, and preserve system accuracy as a separate metric | Apply the MRA080 values to smaller rings or compare resolution bits as accuracy; this is not equivalent to the RLS AksIM-4 HDSL MCD06 system spec |
| RLS AksIM-4 HIPERFACE DSL (current MCD06 catalogue variant) | RLS's current products catalogue lists 20/22-bit variants, 53/80 mm rings, system accuracy ±0.006° (MC053) / ±0.004° (MC080), readhead -40°C to +115°C and ring -40°C to +125°C. The catalogue entry is the current MCD06 configuration; the individual HDSL product-detail page still links legacy MCD05 documentation, so freeze the MCD06 datasheet revision and exact ordering code for formal comparison. | Primary magnetic absolute comparator for the HIPERFACE DSL integration case; match the 80 mm ring and freeze the MCD06 part/data-sheet revision | Calibrated/uncalibrated installed accuracy, or a guarantee for every AksIM-4 variant |
| HEIDENHAIN KCI 120 Dplus | Dual inductive absolute encoder; 2026 product information specifies system accuracy ±40 arcsec on output-side Encoder A (20-bit, ≤6,000 rpm) and ±120 arcsec on motor-side Encoder B (19-bit, ≤15,000 rpm); EnDat 2.2; operating temperature -40°C to +115°C. Accuracy is stated at the specified circular-scale radial runout. | Industrial inductive dual-channel comparator; useful for architecture, speed, temperature and runout-resilience comparisons | A single generic ±40″ claim for both axes, a magnetic encoder, or a claim that the encoder itself is SIL 3 |
| RLS AksIM-4 Dual Concentric | Two absolute channels in one encoder; 19-bit input and output; stated system accuracy up to ±0.010°; -40°C to +105°C | Dual-sensor robot-joint feedback and compact dual-measurement architectures | A single-channel Hall latch |
| Renishaw RESOLUTE ETR (RESA30) | Optical absolute rotary system; typical installed accuracy ±12.7 arcsec at 52 mm, ±4.3 arcsec at 200 mm and ±1.9 arcsec at 550 mm; corresponding maximum reading speeds 18,000, 4,750 and 1,700 rpm. Resolution to 32-bit is explicitly stated to be below the encoder noise floor. | Optical installed-angle/metrology reference and independent rotor-angle truth source | A direct magnetic BOM/environment competitor without application normalization |
| Renishaw REXA30 | Ultra-high-accuracy optical absolute ring; with two RESOLUTE readheads, total installed accuracy ±2 arcsec at ≤57 mm, ±1.5 arcsec at 75 mm and ±1 arcsec at ≥100 mm; SDE ±0.04 arcsec; repeatability 0.01 arcsec; up to 8,500 rpm and +80°C | Optical metrology ceiling for high-accuracy angle truth and an independent angular reference | Direct magnetic competitor at matched cost, temperature or installation constraints |
| Magpylib Material Response 0.15.0 | 25 Nov 2026 release adds analytical and face-charge WASM field kernels checked against shared 50-digit references; face-charge runs through JAX/PyTorch with JIT and gradients; changelog reports a cylinder far-axis stability regression at 576 radii where the previous formula could lose up to 29% of digits | Open field-kernel, differentiable face-charge and numerical-stability reference | A replacement for nonlinear/anisotropic FEM or physical metrology |

Sources:
- TI TMAG5110/TMAG5111 product page: https://www.ti.com/product/TMAG5111
- TI TMAG511x datasheet Rev D: https://www.ti.com/lit/ds/symlink/tmag5111.pdf
- RLS AksIM-2 official product page: https://www.rls.si/eng/aksim-2-off-axis-rotary-absolute-encoder
- RLS AksIM-4 standard family official product page: https://www.rls.si/eng/aksim-4-off-axis-rotary-absolute-encoder
- RLS AksIM-4 HIPERFACE DSL official product page (current page links MCD06): https://www.rls.si/eng/aksim-hdsl-rotary-absolute-magnetic-encoder
- RLS AksIM-2 official product page (20-bit, self-calibration, ±0.004° to ±0.020° system accuracy range): https://www.rls.si/eng/aksim-2-off-axis-rotary-absolute-encoder
- RLS AksIM-2 datasheet MBD01 Issue 15 (13 May 2026; size-by-size system/calibrated-installed/uncalibrated-installed accuracy, speed and bandwidth): https://www.rls.si/cn_tw/fileuploader/download/download/?d=0&id=267&title=Data+sheet%3A+AksIM-2%E2%84%A2+off-axis+rotary+absolute+magnetic+encoder+%28MBD01%29
- RLS current products catalogue (MCD06 HDSL: 20/22-bit, 53/80 mm, ±0.006°/±0.004°): https://www.rls.si/eng/products
- RLS AksIM-4 HDSL detail page (currently links the legacy MCD05 datasheet despite the newer MCD06 catalogue listing): https://www.rls.si/eng/aksim-hdsl-rotary-absolute-magnetic-encoder
- RLS MCD05 Issue 02 datasheet (legacy revision; 20-bit, system accuracy ±0.008° MC053 / ±0.007° MC080, calibrated typical installed ±0.013°/±0.012°): https://www.rls.si/eng/fileuploader/download/download/?d=0&id=540&title=Data+sheet%3A+AksIM-4%E2%84%A2+HIPERFACE+DSL%C2%AE+Absolute+Rotary+Magnetic+Encoder+%28MCD05%29
- RLS AksIM-4 Dual Concentric official product page: https://www.rls.si/eng/aksim-dual
- HEIDENHAIN KCI 120 Dplus product information PDF (12/2021): https://news.heidenhain.com/fileadmin/pdf/en/01_Products/Produktinformationen/PI_KCI120_Dplus_ID1357659_en.pdf
- Renishaw RESOLUTE ETR datasheet: https://www.renishaw.com/resourcecentre/download/data-sheet-resolute-etr-extended-temperature-range-absolute-encoder--133590?userLanguage=en
- Renishaw REXA30 datasheet: https://www.renishaw.com/resourcecentre/download/data-sheet-rexa-ultra-high-accuracy-absolute-angle-encoder--104086?userLanguage=en
- Magpylib Material Response 0.15.0 changelog (576-radius cylinder-axis stability case): https://magpylib-material-response.readthedocs.io/en/stable/changelog.html
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

For an 80 mm off-axis magnetic design, the strongest currently documented same-family challenge is RLS AksIM-2 MRA080: 20-bit maximum, ±0.004° system accuracy, typical installed ±0.009° calibrated / ±0.029° uncalibrated, up to 4,700 rpm, with the manufacturer reporting no thermal drift. The system-accuracy footnote says the figure corresponds to installation with two readheads or best possible after self-calibration. This gives us an actual installed-accuracy comparator, not merely a system headline.

The AksIM-4 HDSL MCD06 remains a second primary challenge: MC053 (53 mm, system accuracy ±0.006°) and MC080 (80 mm, ±0.004°). Freeze the exact MCD06 data-sheet revision and ordering code because the detail page still links legacy MCD05 documentation. The older MCD05 Issue 02 datasheet is materially different: 20-bit, ±0.008°/±0.007° system accuracy and typical calibrated installed accuracy ±0.013°/±0.012°.

For the 80 mm challenge, proposed research targets are ≤±0.004° system accuracy for parity, ≤±0.003° system accuracy as the first competitive stretch, and ≤±0.007° typical calibrated installed accuracy (better than the MRA080 ±0.009° published typical). Also aim for ≤±0.020° typical uncalibrated installed accuracy as a separate stretch target against ±0.029°. For the 53 mm HDSL build, target ≤±0.006° parity and ≤±0.005° stretch. These are proposed targets, not achieved results. Match ring diameter, readhead count, air gap, runout/eccentricity, self-calibration, speed, bandwidth and thermal protocol. Keep system accuracy, calibrated installed accuracy and uncalibrated installed accuracy separate. Never relabel a system-accuracy figure as an installed-field result.

Use HEIDENHAIN KCI 120 Dplus as an inductive dual-encoder comparator. The current cited product-information PDF calls the metric **system accuracy**, and specifies ±40 arcsec on output-side Encoder A at 20-bit/≤6,000 rpm and ±120 arcsec on motor-side Encoder B at 19-bit/≤15,000 rpm. It uses EnDat 2.2 and is specified for -40°C to +115°C. The accuracy is conditional on the stated radial runout of the circular scale. Do not collapse the two channels into one generic accuracy figure or label the number "positioning accuracy".

Use Renishaw RESOLUTE ETR as an optical metrology reference, not a direct magnetic competitor. Add the Renishaw REXA30 as a high-accuracy optical ceiling: dual-readhead total installed accuracy ±2 arcsec at ≤57 mm, ±1.5 arcsec at 75 mm and ±1 arcsec at ≥100 mm. These are not like-for-like cost/environment targets, but they provide an external angular-truth benchmark. Report resolution, system accuracy, installed accuracy, repeatability, hysteresis, runout sensitivity, temperature drift, speed, bandwidth, external-field susceptibility, calibration burden, power, BOM and serviceability separately.

### E. Active metrology

Compare uniform grid, seeded random, field-variance, cross-solver disagreement, sensitivity-only, decision-directed lower-bound planner and a full-map oracle. Freeze the blind holdout before adaptive sampling.

Report measurements and acquisition seconds to stabilize the pass/fail decision, worst blind-holdout functional error, false accepts/rejects, missed risk regions, probe re-registration and calibration overhead. Initial target: at least 2× fewer adaptive observations than the uniform-grid baseline at the same blind-holdout decision error, with zero blind-data leakage. This is an empirical benchmark target, not an assumed benefit.

A traceability chain has to identify the measurand, result and uncertainty, and each calibration reference; traceability alone does not establish that the uncertainty is fit for purpose. See NIST's metrological traceability policy: https://www.nist.gov/metrology/metrological-traceability.

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
- [#7658](https://github.com/Luminous-Dynamics/symtropy/issues/7658) / [#7674](https://github.com/Luminous-Dynamics/symtropy/pull/7674): interval branch-and-bound for a single scalar objective and robust margin. v0.1 keeps one unsplit nuisance box and is stacked on interval AD.
- [#7667](https://github.com/Luminous-Dynamics/symtropy/issues/7667) / [#7678](https://github.com/Luminous-Dynamics/symtropy/pull/7678): measurement-informed, cell-wide physical model-discrepancy envelopes; v0.4 can construct the residual-Jacobian matrix from three interval-AD certificates (Bx/By/Bz), conditional on the expression identities and model-to-physical applicability.
- [#7679](https://github.com/Luminous-Dynamics/symtropy/issues/7679) / [#7680](https://github.com/Luminous-Dynamics/symtropy/pull/7680): qualify rigorous elementary-function enclosures for trigonometry and angle functions. The candidate probe pins `scioxide-validated 0.1.0`; it is an evaluation harness, not a production backend switch. Exact-head run #45922458978 passed Linux/macOS/Windows against the nine authored provider-semantics tests.
- [#7681](https://github.com/Luminous-Dynamics/symtropy/issues/7681) / [#7682](https://github.com/Luminous-Dynamics/symtropy/pull/7682): branch-cut-free phasor cosine-margin evaluation, stacked on interval AD. The first run reached Clippy and reported one indexed parameter-validation loop; fixed at current head `f71d7a9...`, with exact-head rerun #45923365349 queued.

These are research implementations, not verified product capabilities or SOTA results. PR #7655 passed its normal and dedicated magnetic-research Linux/macOS/Windows workflows. PR #7674 passed exact-head run #45920175138 on all three platforms. PR #7678 passed exact-head run #45921346634 on all three platforms, including 16 physical-discrepancy tests and 36 interval-AD dependency tests. PR #7680's probe passed its nine authored tests and three-platform main CI. PR #7682 is not yet qualified until the corrected exact-head matrix completes. The older #7675 PR is superseded.


## 9. 27 November 2026 addendum: AksIM-2 matched magnetic baseline

The official RLS MBD01 Issue 15 datasheet (13 May 2026) sharpens the magnetic comparison: the MRA080 80 mm ring has 20-bit maximum resolution, system accuracy ±0.004°, typical calibrated installed accuracy ±0.009°, and typical uncalibrated installed accuracy ±0.029°. It specifies a 4,700 rpm maximum for size 080, 9 kHz bandwidth, 18 kHz sampling, up to 54 kHz refresh and no thermal accuracy drift. The footnote says its system-accuracy value represents two-readhead installation or best possible self-calibration. The table also lists MRA064 at ±0.004° system accuracy, ±0.010° calibrated installed and ±0.032° uncalibrated installed. This means our best near-size magnetic comparator is not just MCD06 MC080; it is AksIM-2 MRA080 with its documented installed-error figures.

AksIM-2 also exposes a physical-resilience constraint worth testing: the same datasheet says external fields above 20 mT can temporarily malfunction the encoder and above 50 mT can permanently damage the ring. Its product page advertises external-field immunity, but the datasheet's numeric limits are the correct qualification inputs. A magnetic product cannot be compared on nominal angular accuracy alone; characterize the field-strength envelope, direction, frequency, exposure duration and recovery state.

The practical competitive opportunity is therefore a matched 80 mm test that reports both calibrated and uncalibrated installed accuracy, thermal shift, maximum tracking speed/bandwidth, ring/readhead runout sensitivity, external-field susceptibility, repeatability and hysteresis. A first stretch target of ≤±0.007° calibrated installed accuracy is measurable against the published ±0.009° MRA080 typical, while the ≤±0.003° system figure competes against the current ±0.004° magnetic benchmark. Neither target is a claim of current Symtropy performance.

## 8. 27 November 2026 addendum: what the industry figures actually mean

Do not compare these encoder values as if "accuracy" were one interchangeable quantity:
- RLS MCD06 figures are manufacturer-stated **system accuracy**. The older MCD05_02 datasheet is a separate revision and gives different figures; the current HDSL detail page still linking MCD05 is a documentation-lifecycle ambiguity that must be resolved by obtaining the exact MCD06 PDF/ordering code before formal procurement or head-to-head testing.
- HEIDENHAIN KCI 120 Dplus also labels its figures **system accuracy**, but separately for output-side Encoder A and motor-side Encoder B. Those channels have different bit depth, maximum speed and accuracy; the figures assume the stated circular-scale radial runout.
- Renishaw RESOLUTE ETR publishes **typical installed accuracy** dependent on RESA30 diameter and speed. Its 32-bit resolution is below the noise floor, so resolution bits are not a proxy for accuracy.
- Renishaw REXA30's ≤±1 arcsec headline is a **total installed accuracy** claim for the dual-readhead configuration at ring diameters ≥100 mm; its smaller-ring accuracy varies as detailed in the datasheet. It is an optical metrology ceiling, not a like-for-like magnetic cost/environment target.

The Symtropy benchmark must therefore compare matched ring diameter, installation/runout, readhead count, self-calibration, speed and thermal envelope; report system accuracy, installed accuracy, repeatability, hysteresis, SDE/noise, and resolution in separate fields. The primary differentiation opportunity is not simply increasing bits: it is proving worst-case functional margin over the declared physical cell, manufacturing tolerance, temperature, runout and stray-field envelope while preserving a traceable measurement chain. See NIST's traceability policy: https://www.nist.gov/calibrations/traceability.
