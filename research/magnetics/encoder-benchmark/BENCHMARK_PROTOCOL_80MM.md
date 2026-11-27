# Preregistered 80 mm magnetic absolute-encoder benchmark protocol

**Protocol ID:** MAG-ENCODER-BENCH-001  
**Revision:** 0.1-draft  
**Status:** engineering protocol; no hardware result claimed  
**Issue:** [#7708](https://github.com/Luminous-Dynamics/symtropy/issues/7708)

## 1. Decision question

Can a magnetic absolute-angle encoder and its complete installed measurement system deliver a better traceable accuracy/robustness/cost envelope than current 80 mm industrial alternatives, while maintaining a conservative spatial functional-margin certificate?

The primary matched comparison is:

- **Reference magnetic baseline:** RLS AksIM-2, MRA080 ring + MB080 readhead, with the actual interface, revision and calibration mode frozen from the purchased parts.
- **Second magnetic comparator:** RLS AksIM-4 HIPERFACE DSL, MC080/MCD06. This remains provisional until the supplier provides the exact current MCD06 part number and PDF revision. The current catalogue lists MCD06 variants and MC080 system accuracy ±0.004°, but an individual HDSL product page has been observed linking legacy MCD05 documentation. Do not substitute MCD05 measurements for MCD06.
- **Independent angular truth:** calibrated rotary reference with an uncertainty budget. An optical encoder such as Renishaw RESOLUTE/REXA30 may be part of the reference chain after verifying its own ring, installation, calibration and uncertainty. It is not treated as a like-for-like magnetic competitor.

## 2. Manufacturer baseline — preserve definitions

RLS MBD01 Issue 15 (13 May 2026) lists MRA080 as:
- maximum resolution: 20 bit;
- system accuracy: ±0.004°;
- typical installed accuracy, calibrated: ±0.009°;
- typical installed accuracy, uncalibrated: ±0.029°;
- maximum speed at size 080: 4,700 rpm;
- bandwidth: 9 kHz; sampling: 18 kHz; refresh: up to 54 kHz;
- operating temperature: -40°C to +105°C;
- stated thermal drift: none (accuracy does not change with temperature);
- hysteresis and repeatability: less than one unit of resolution.

The datasheet footnote defines its system accuracy as installed accuracy when using two readheads, or the best possible result after self-calibration. The uncalibrated installed result is affected by mounting precision, eccentricity and bearing wander. These figures must not be collapsed into one accuracy column.

The datasheet states external magnetic fields above 20 mT temporarily cause encoder malfunction; fields above 50 mT may permanently damage the ring. This is a hazard boundary, not a test target to exceed. Non-destructive interference tests must be separately risk-reviewed.

## 3. Standards-informed method — not a conformity claim

Use these published standards as methodological references and review the licensed texts before claiming conformance:

- **ISO 230-2:2014 + Amd 1:2016:** repeated direct measurement for positioning accuracy/repeatability of NC axes; its abstract states the methods apply to rotary axes as well as linear axes and includes uncertainty estimation references to ISO/TR 230-9.
- **ISO 230-7:2015:** rotary-axis geometric error motion and speed-induced axis shifts. It explicitly distinguishes angular positioning accuracy and thermal effects, which are handled by other parts.
- **ISO 230-3:2020:** environmental temperature variation and thermal effects associated with rotating spindles and rotary motion.

This is an encoder/system benchmark inspired by relevant test-code methods. It is not an assertion that either encoder or this protocol is ISO-certified.

## 4. Preregistration and configuration freeze

Before collecting comparison data, freeze:
- exact product part number, datasheet issue/revision, firmware, readhead count, interface and communication timing;
- ring and readhead serials, manufacturing lot, hardware revision and magnetisation/calibration configuration where provided;
- actual ring OD/ID, shaft/fixture geometry, fasteners, mounting surfaces, torque, runout, eccentricity, tilt, air gap, bearing wander and bearing specification;
- cable type/length, power supply, grounding, shields, data acquisition equipment and sampling clocks;
- calibration state and procedure, including whether AksIM-2 self-calibration is enabled and whether two heads are used;
- test order/randomisation seed, measurement grid, repetitions, outlier/rejection rules, filters, interpolation and reduction-code digest;
- blind-holdout angular sectors, environment/speed cases and access controls.

For AksIM-4 MC080/MCD06, the protocol remains **not ready for a matched comparison** until the current supplier document and ordering code are frozen. Keep all unknowns as unknowns.

## 5. Common mechanical and environment envelope

### Geometry and install
The target is an 80 mm class off-axis magnetic system. Record the actual dimensions; do not infer dimensional identity from the product code alone. Match the application fixture and installation method where feasible. Characterise:
- radial and axial runout;
- ring eccentricity and ovality;
- readhead air gap and angular alignment;
- bearing wander, shaft deflection and mounting stiffness;
- cable routing, vibration and ambient magnetic-field background.

Use a baseline installation and a preregistered tolerance sweep. Select tolerance points from supplier limits and the actual application drawing, not arbitrary post-hoc values.

### Temperature
The common head-to-head temperature range is -40°C to +105°C, limited by the published AksIM-2 MRA080 operating range. AksIM-4 HDSL may have a wider readhead/ring temperature rating, but any additional test beyond +105°C is a separate, non-like-for-like product-characterisation leg unless the AksIM-2 configuration is independently rated and tested there.

Record sensor, ring, shaft, housing and ambient temperatures; dwell/soak criterion; ramp rate; time since stabilisation; and reference-system drift. Capture both steady-state and transition behaviour.

### Speed and dynamics
Test common preregistered speeds up to the lower verified rated limit of both exact configurations. For MRA080, the current datasheet lists 4,700 rpm. Do not test MC080/MCD06 above its verified supplier rating. Measure:
- speed error and tracking loss;
- usable bandwidth, sample/update rate, data age, latency and jitter;
- interface behaviour under load and packet/clock faults;
- thermal state and angular error throughout acceleration/deceleration.

Do not equate nominal resolution, serial baud rate or refresh rate with closed-loop bandwidth.

## 6. Angular measurement procedure

The internal proposal is a full 360° map in both directions:
- preregister an angular grid and hold it fixed;
- take repeated clockwise and counter-clockwise traversals;
- include multiple complete revolutions at each selected steady speed;
- randomise product order and test order to reduce warm-up/time confounds;
- repeat after each installation/calibration state;
- retain raw encoder output and independent reference angle at native timestamps.

The exact number of points, revolutions and speeds must be frozen in the preregistration before collection. A nominal 1° grid (360 positions) is a practical first engineering map, but it is not asserted to be the ISO-prescribed grid. Add higher-resolution targeted harmonic measurements only as separately labelled diagnostic runs; do not let them replace the preregistered holdout.

Compute circular angle error for reporting:
`e(θ) = wrap(θ_encoder - θ_reference)`.
The wrap is a diagnostic/reporting operation for angular error, not the functional-margin decision path. The separate phasor-space margin remains branch-cut-free.

## 7. Calibration and holdout discipline

Run and report distinct modes:
1. uncalibrated single-head installation;
2. calibrated/self-calibrated single-head installation where supported;
3. two-head installation only where the exact product supports it; for AksIM-2, two readheads are mounted 180° apart per its datasheet.

Do not mix these measurements. Preserve the calibration dataset, coefficient/model digest, procedure, operator, duration, rotation span and pass/fail state.

Partition angular positions and environment/speed cases before calibration:
- calibration/fit set;
- adaptive exploration set;
- sealed blind-holdout set;
- post-selection qualification set.

No blind-holdout data may enter calibration, model selection, adaptive probe planning or threshold changes. Any access to blind data invalidates that run as an independent holdout.

## 8. Metrics — independent columns

Report at least:
- vendor system-accuracy specification and its footnote/conditions;
- measured uncalibrated installed maximum absolute error;
- measured calibrated installed maximum absolute error;
- RMS error and full error distribution;
- repeatability and hysteresis;
- SDE/noise/jitter and missing/invalid samples;
- harmonic spectrum, explicitly identifying eccentricity/mounting terms, ovality, mounting-hole harmonics and other observed components;
- thermal drift and post-transition recovery;
- speed, bandwidth, sampling/update rate, data age, latency and jitter;
- runout/eccentricity/air-gap sensitivity;
- external-field magnitude/direction/frequency susceptibility and recovery;
- diagnostics, calibration time, installation time, power, BOM and service burden.

Never use bits as a substitute for accuracy. Never compare a manufacturer “typical installed” number to a different product's “system accuracy” headline without a separate label and explanation.

## 9. Metrology and uncertainty

The angular reference chain must identify:
- measurand and reference instrument;
- calibration certificate ID, issue, range and uncertainty;
- traceability chain and intermediate standards;
- reference sampling/latency and synchronization uncertainty;
- alignment, runout and fixture uncertainty;
- temperature stability and thermal expansion;
- data-reduction and rounding contributions.

Report combined standard uncertainty and expanded uncertainty with the stated coverage factor. State the method and assumptions used to combine contributions. A traceability link alone does not prove the uncertainty is suitable for the target. As an internal design objective, aim for expanded reference uncertainty substantially below the narrowest system-accuracy claim; a provisional U(k=2) ≤0.001° target gives a 4:1 ratio to a ±0.004° claim, but the actual uncertainty budget must substantiate it. If it cannot, classify that comparison as metrologically inconclusive.

## 10. External magnetic field and destructive-risk boundary

Record the ambient magnetic-field vector and spectrum at the encoder. For non-destructive qualification, define a risk-reviewed field sweep that stays within safe operating constraints. The MRA080 MBD01 datasheet warns that fields above 20 mT can temporarily malfunction and above 50 mT may permanently damage the ring. Do not expose a specimen to destructive levels as part of routine comparison. Any failure-injection leg requires separate written approval, specimen isolation, containment and explicit sacrificial status.

## 11. Specimen strategy

Stage 1 — engineering characterisation: begin with at least five independent assemblies per frozen product/configuration to discover variance, installation sensitivity and obvious failure modes. This is a screening cohort, not a production-population claim.

Stage 2 — confirmatory: choose the specimen count from Stage 1 variance and a preregistered confidence/power objective. Include multiple production lots where feasible. Do not declare a production distribution based on a small engineering sample.

Report per-unit and pooled values, with between-unit and within-unit variance kept separate. Do not hide a poor specimen by averaging it with better units.

## 12. Research targets — not achieved results

For a matched 80 mm magnetic system:
- system accuracy parity: ≤±0.004°;
- system accuracy stretch: ≤±0.003°;
- calibrated installed target: ≤±0.007° against MRA080's published typical ±0.009°;
- uncalibrated installed stretch: ≤±0.020° against MRA080's published typical ±0.029°;
- zero false-safe spatial functional-margin classifications on the preregistered challenge set, including uncertainty and physical-discrepancy intervals.

A target is not a result. Any “better than industry” conclusion requires the measured uncertainty interval to support the difference on like-for-like configurations.

## 13. Report/evidence bundle

Each result bundle must include:
- protocol revision and preregistration digest;
- product part numbers, serials, lots and datasheet digests;
- installation drawings/photos and mechanical survey;
- calibration files and holdout partition digest;
- raw encoder/reference samples and acquisition metadata;
- reference calibration and uncertainty budget;
- test environment, speed, temperature and external-field records;
- reduction software version/source commit/build digest;
- analysis outputs, plots and machine-readable metrics;
- deviations, failures, restarts and exclusions;
- evidence role: calibration, adaptive exploration, blind holdout or post-selection qualification.

The result is not accepted if a field is silently substituted, a measurement is manually discarded without a preregistered rule, or an evidence role is changed after data collection.

## 14. Benchmark comparisons

Compare the same dataset and blind-holdout set against:
1. uniform-grid sampling;
2. fixed seeded random sampling;
3. sensitivity-only sampling;
4. cross-solver disagreement sampling;
5. decision-directed lower-bound planning;
6. full-map oracle (diagnostic ceiling only, never a deployed algorithm).

Report physical observations and acquisition time required to reach the same false-safe rate, unresolved volume, false rejects, missed-risk regions and calibration/re-registration overhead. The aspirational 2× fewer measurements target is only meaningful at the same blind-holdout false-safe rate.

## 15. References

- RLS AksIM-2 MBD01 Issue 15 datasheet, 13 May 2026: https://www.rls.si/eng/fileuploader/download/download/?d=0&id=267&title=Data+sheet%3A+AksIM-2%E2%84%A2+off-axis+rotary+absolute+magnetic+encoder+%28MBD01%29
- RLS AksIM-2 product: https://www.rls.si/eng/aksim-2-off-axis-rotary-absolute-encoder
- RLS current products catalogue / MCD06 listing: https://www.rls.si/eng/products
- HEIDENHAIN KCI 120 Dplus product information: https://news.heidenhain.com/fileadmin/pdf/en/01_Products/Produktinformationen/PI_KCI120_Dplus_ID1357659_en.pdf
- Renishaw RESOLUTE ETR datasheet: https://www.renishaw.com/resourcecentre/download/data-sheet-resolute-etr-extended-temperature-range-absolute-encoder--133590?userLanguage=en
- Renishaw REXA30 datasheet: https://www.renishaw.com/resourcecentre/download/data-sheet-rexa-ultra-high-accuracy-absolute-angle-encoder--104086?userLanguage=en
- ISO 230-2:2014 (confirmed current in 2025; includes rotary-axis positioning accuracy/repeatability methods): https://www.iso.org/standard/55295.html
- ISO 230-7:2015 (confirmed current in 2026; rotary-axis error motion and speed-induced shifts): https://www.iso.org/standard/56624.html
- ISO 230-3:2020 (confirmed current in 2026; environmental and rotary-motion thermal effects): https://www.iso.org/standard/73291.html
- NIST metrological traceability: https://www.nist.gov/metrology/metrological-traceability


## 16. Comparator taxonomy and current external frontier (reviewed 27 November 2026)

Do not create one misleading leaderboard from products that solve different measurement problems.

### A. Magnetic absolute-angle encoders — primary product comparison

**RLS AksIM-2 MRA080 / MB080** is the frozen baseline. MBD01 Issue 15 (13 May 2026) reports ±0.004° system accuracy, typical ±0.009° installed accuracy after calibration and ±0.029° uncalibrated. Keep the three definitions separate and use the exact product/install/calibration configuration in the result.

**RLS AksIM-4 HIPERFACE DSL MC080 / MCD06** remains a provisional, potentially stronger magnetic comparator. Current catalogue/product snippets list an MC080 ±0.004° system-accuracy variant, but the accessible product page has also served legacy MCD05 documentation. RLS's MCD05 document says that revision is not recommended for new designs and points new applications to MCD06. Until the supplier provides the exact MCD06 PDF revision and order code, record the public ±0.004° figure as a provisional catalogue claim only; do not assert datasheet-verified parity or substitute MCD05 limits for MCD06.

### B. Optical absolute encoders — metrology ceiling, not a like-for-like product claim

Renishaw RESOLUTE installed-accuracy examples vary with ring diameter: typical ±12.7 arcsec at 52 mm, ±4.3 arcsec at 200 mm and ±1.9 arcsec at 550 mm. The 200 mm and 550 mm figures are approximately ±0.00119° and ±0.00053°. These are optical installed-system figures with their own ring and installation assumptions; they are useful as a metrology ceiling and as candidates for the independent angular-reference chain, not as proof that an 80 mm magnetic assembly is directly comparable.

The RESOLUTE documentation also states that 32-bit resolution is below the system noise floor. This is a useful reminder that count depth is not effective accuracy. If a two-readhead REXA30 optical arrangement is used as the reference, freeze its exact ring, readhead, calibration, mounting and uncertainty contribution; published variants can reach ±1 arcsec total installed accuracy for supported diameters.

### C. Incremental Hall latches — separate digital edge-integrity benchmark

TI TMAG5111 is a two-dimensional dual-channel Hall-effect latch for speed/direction sensing, with a published 40 kHz bandwidth and typical operate-point variants around 1.4 mT and 3 mT. It is an appropriate comparator for pole-edge detection, quadrature integrity, speed/direction and dropout/threshold margin. It is not an absolute-angle encoder and must not appear in the same absolute accuracy ranking as AksIM or RESOLUTE.

### D. Magnetic field solvers and differentiation — computational comparison, not product accuracy

Magpylib Material Response 0.15.0 (released 25 November 2026) expands the computational baseline: its release notes describe WebAssembly field kernels checked against shared high-precision reference files and JAX/PyTorch differentiation for its Python face-charge kernel, as well as cylinder far-field improvements. This makes kernel duplication an unconvincing differentiator.

Freeze Magpylib 0.15.0 as one external computational arm, alongside:
1. analytic point-dipole and finite-cuboid known answers;
2. an independently configured FEM solver for challenge geometries/materials;
3. validated interval-AD and physical-discrepancy enclosures;
4. sealed measured field maps from the traceable metrology chain.

Report field-vector residual, gradient agreement, interval containment/coverage on exact analytic cases, runtime and memory. Do not compare solver field error to encoder system accuracy. The competitive contribution of this programme is the verified transfer from model/physical field to installed sensor function and decision—not a claim to have invented magnetostatic kernels or automatic differentiation.

### E. Measurement uncertainty and acceptance

The independent angular-reference budget must separately account for test-encoder resolution, reference resolution, reference calibration uncertainty, reference interpolation error, alignment/eccentricity, repeatability and other Type A/Type B contributions. EURAMET's rotary-encoder calibration guide is a useful method reference. Report combined standard uncertainty and expanded uncertainty with stated coverage factor and assumptions; a calibration certificate alone does not prove adequate comparison uncertainty.

### Sources to freeze with the preregistration

- RLS AksIM-2 MBD01 Issue 15: https://www.rls.si/eng/fileuploader/download/download/?d=0&id=267&title=Data+sheet%3A+AksIM-2%E2%84%A2+off-axis+rotary+absolute+magnetic+encoder+%28MBD01%29
- RLS current catalogue / MCD06 product family: https://www.rls.si/eng/products
- TI TMAG5111 data sheet: https://www.ti.com/lit/ds/symlink/tmag5111.pdf
- Renishaw RESOLUTE ETR data sheet: https://www.renishaw.com/resourcecentre/download/data-sheet-resolute-etr-extended-temperature-range-absolute-encoder--133590?userLanguage=en
- Renishaw REXA30 data sheet: https://www.renishaw.com/resourcecentre/download/data-sheet-rexa-ultra-high-accuracy-absolute-angle-encoder--104086?userLanguage=en
- Magpylib 0.15.0 release and artifact record: https://pypi.org/project/magpylib/0.15.0/
- Magpylib changelog: https://magpylib.readthedocs.io/en/latest/changelog.html
- EURAMET Calibration Guide No. 23, rotary encoders: https://www.euramet.org/publications-media-centre/calibration-guidelines
