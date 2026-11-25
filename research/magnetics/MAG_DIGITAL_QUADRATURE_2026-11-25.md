# Digital magnetic encoder quadrature — research dossier, 25 November 2026

## Why this is a separate benchmark

The field-to-angle evaluator in #7411/#7413 estimates continuous electrical phase from atan2(By, Bx). It is appropriate for a vector sensor / continuous angle signal, but it does not describe an incremental Hall-latch encoder.

A digital incremental encoder has a different causal chain:

rotor angle -> radial/tangential field -> sensor-axis projection -> Hall hysteresis / BOP / BRP -> digital A/B states -> Gray-code transitions -> counted position.

The sensor's magnetic operating/release thresholds, their mismatch, sensor placement, field strength and speed all influence the result. More poles are not unconditionally better: the field available at the sensor decreases as pole pitch shrinks, and the permitted air-gap/placement tolerance becomes tighter.

## Concrete reference: TI SBAA449B

Texas Instruments application note SBAA449B (revised October 2021), “Reducing Quadrature Error for Incremental Rotary Encoding Using 2D Hall-Effect Sensors,” supplies a reproducible 20-pole bonded-ring case:
- NeoBond12M ring: 25.5 mm outer diameter and 21.5 mm inner diameter.
- 20 magnetic poles = 10 pole pairs.
- 36 degrees mechanical per electrical cycle; 9 degrees mechanical for 90-degree electrical separation.
- Two separate Hall devices at 3.25 mm sensor distance; radial field peak just above 8 mT.
- A stated BOP mismatch scenario (1 mT against 7 mT) produces up to 4.9 degrees mechanical quadrature error in the note's analysis.
- A single-package 2D Hall arrangement uses radial/tangential components; the note reports about 1.35 degrees after its stated worst-case threshold mismatch analysis.

These are document-specific design calculations, not measured universal SOTA. The same TI note also includes physical test data for sensor radial positioning and reports a position sweet spot around 0.11 mm from the ring's outer face under its stated geometry; transitions failed beyond approximately 3.85 mm in that test setup. The measured position sweep and the 1.35-degree analysis must remain separate evidence items.

TI's TMAG5110 / TMAG5111 product family is a relevant 2D dual Hall-latch comparator. TMAG5110 provides independent latch outputs; TMAG5111 provides speed/direction outputs. The product family publishes typical operating-point and bandwidth values; these must not be mistaken for guaranteed worst-case limits. Pin exact part suffix, datasheet revision, temperature and threshold conditions before use.

## Executable v0.1

The new zero-dependency Rust crate lives at research/magnetics/digital-quadrature/.

It models:
- independent latch thresholds and hysteresis states;
- discrete sensors at different field positions or one integrated two-axis sensor at a shared local field vector;
- full-revolution sampled traces and explicit initial states;
- A/B Gray state sequence, illegal two-bit changes, direction reversal and missing/excess transitions;
- per-channel rising/falling counts;
- angle/time sample brackets for each transition;
- a caller-declared edge-rate budget derived from pole pairs and RPM.

For an integrated two-axis sensor, both channels receive one shared local field vector. Any measured deviation from exact axis orthogonality is reported rather than rejected, allowing package alignment error to be part of the design study. The model never invents a transition timestamp between samples. It computes bounded maximum quadrature edge-spacing error from each transition's sample bracket, including the end-to-start wrap gap. A separate spatial-spectrum function resolves harmonics at integer multiples of the pole-pair fundamental and reports THD, unresolved residual and endpoint discontinuity for each projected latch input. The spectral metrics are diagnostic, not a substitute for quadrature accuracy. The edge-rate ceiling is an explicit benchmark policy, not a vendor bandwidth equivalence.

## Next scientific step

1. Reproduce the TI 20-pole ring's field map at the declared sensor coordinate and air gap.
2. Use the field map as input to this state-machine evaluator with exact device BOP/BRP assumptions.
3. Reproduce the discrete-latch threshold mismatch case.
4. Reproduce the integrated 2D Hall analysis separately, keeping its shared sensor position / orthogonal axes.
5. Reproduce the published physical radial-position sweep as a separate measured dataset.
6. Then perform a matched tolerance tournament: ring geometry, magnetization depth, air gap, sensor radial position, eccentricity, tilt, pole pitch, BOP/BRP spread, temperature and RPM.
7. Compare candidate manufacturing-aware patterns against the same conventional ring, sensor and reference encoder.

## Benchmark scoreboards

Keep these independent:
- continuous vector-phase angle error (#7413);
- digital quadrature edge integrity (this crate);
- sensor analogue threshold margin and hysteresis;
- speed / response / timing against the exact part's guaranteed specifications;
- manufacturing robustness and accepted-part cost.

Do not compare a vector-sensor atan2 angle error with digital latch quadrature error or with an absolute on-axis encoder accuracy number.

## Evidence status

- Latch / quadrature evaluator: authored; unit tests not yet run by CI.
- TI ring numerical reproduction: not yet performed.
- Exact TI threshold / response model reproduction: not yet performed.
- Physical test reproduction: literature source identifies a physical sweep; our own independent reproduction not performed.
- New ring fabrication or measurement: not performed.
- SOTA superiority: not demonstrated.

## References

- TI SBAA449B, https://www.ti.com/lit/an/sbaa449b/sbaa449b.pdf
- TI TMAG5111, https://www.ti.com/product/TMAG5111
- TI TMAG511x datasheet, https://www.ti.com/lit/ds/symlink/tmag5111.pdf
- TI SBOA200B, https://www.ti.com/document-viewer/lit/html/SBOA200B
- Parent issue #7446, https://github.com/Luminous-Dynamics/symtropy/issues/7446

## Finite operating-condition tournament

The executable reference now evaluates multiple traces under one fixed sensor/latch configuration while allowing distinct operating-condition IDs. This is how air-gap, eccentricity, tilt, temperature and field-margin scenarios can be compared without pooling them as one identical condition. Frame, angular-grid ID, sample count and mechanical angle coordinates must match bit-for-bit; trace IDs must be unique; synthetic, solver-prediction, calibrated-measurement and captured-output evidence classes cannot be mixed.

The evaluator preserves every scenario and reports worst-case missing/excess/illegal transitions, direction reversals, bounded quadrature spacing error and transition-bracket width. It emits no probability or manufacturing-yield inference. Use this finite-set layer before any statistically designed repeated-build analysis.
