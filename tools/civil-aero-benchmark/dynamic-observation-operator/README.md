# AER-01 dynamic observation operator v0.1

**Tracking:** [AERO-DYNAMIC-OBSERVATION-001 / #4236](https://github.com/Luminous-Dynamics/symtropy/issues/4236)  
**Reference catalog:** [DPW-8/AePW-4 catalog v0.2 on PR #4137](https://github.com/Luminous-Dynamics/symtropy/pull/4137)  
**Parent structural bridge:** [#4093](https://github.com/Luminous-Dynamics/symtropy/issues/4093)

## What this does

This crate converts the DPW-8 JAXA CRM Test Case 3 channel inventory into a machine-readable comparison-readiness report. It also implements a small typed gate for deciding whether two observation products may be compared pointwise, by RMS, PSD, coherence or modal-validation claim.

It is not a signal-processing engine, CFD/FSI/FEM solver, modal identifier, or source-data importer. It does not compute a spectrum from JAXA data.

## Core model

A simulation field is not the same thing as an instrument observation. A declared observation operator maps a physical prediction into the measurement space:

\`y_hat = M[H(S[x_hat])] + b\`

- \`S\`: spatial sample/mapping operator (sensor locations, surface pixels, coordinate transforms)
- \`H\`: temporal/frequency instrument response (sensor bandwidth, exposure and filters)
- \`M\`: reliability mask and missing-data operator
- \`b\`: admitted calibration/bias transform

Only products that share an admitted quantity, unit, reference frame, spatial mapping, temporal response, mask treatment and bandwidth are directly comparable. Otherwise the software rejects the direct comparison or abstains pending evidence. Inverse filtering is not attempted.

For PSD and coherence, additional metadata is required: sample rate, timebase, synchronization uncertainty, window, segment length, overlap, detrending, estimator, normalization and output units. Missing metadata does not receive defaults.

## DPW-8 Test Case 3 context

The case is optional. The JAXA page reports the 2.16% CRM test article at Mach 0.85, chord Reynolds number approximately 2.27e6, total pressure 12 kPa and angles 3.05, 3.61 and 4.70 degrees.

The catalog distinguishes six measurement families:
- balance coefficients (PSP-simultaneous vs standalone balance);
- wing-root strain-gauge buffet indicator \`C_B\`;
- steady ESP point pressure;
- Kulite point-pressure RMS and PSD;
- uPSP surface mean/RMS/PSD/snapshots;
- official structural deformation (currently pending).

Important source limitations are preserved:
- PSP-simultaneous drag is N/A and stays null.
- \`C_B\` is a buffet-onset indicator, not a structural mode or fatigue metric.
- JAXA attributes the Kulite PSD feature near 9 kHz to wind-tunnel noise.
- uPSP masks unreliable regions and camera-exposure low-pass filtering can make its RMS differ from Kulite RMS.
- Official deformation measurements remain pending; the 10–12 mm wing-tip figure at alpha 4.7 degrees is qualitative only.

JAXA source: https://cfdws.chofu.jaxa.jp/apc/dpw/upsp.html  
AIAA Test Case 3 page: https://www.aiaa-dpw.org/TestCases/testcases.html

The JAXA source page states all rights reserved. The referenced CSV/ZIP files remain linked but unacquired in the catalog. No raw data bytes, calibration, sampling rate, clock alignment, exposure transfer function, PSD window or normalization are claimed here.

## Fail-closed outputs

For the current v0.2 catalog the report must state:
- source admission: NOT_ADMITTED;
- PSD comparison: ABSTAIN due missing sampling/estimator metadata;
- cross-channel coherence: ABSTAIN due missing common timebase/synchronization;
- direct Kulite-vs-uPSP RMS equality: REJECT due different observation operators;
- \`C_B\`: indicator only;
- PSP-simultaneous drag: NULL_NOT_IMPUTED;
- quantitative deformation: NOT_ASSESSED because official measurements are pending;
- structural modal validation: NOT_ASSESSED.

## Run locally

From repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/dynamic-observation-operator/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/dynamic-observation-operator/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/dynamic-observation-operator/Cargo.toml > /tmp/aero-dynamic-observation-audit.json
python3 tools/civil-aero-benchmark/dynamic-observation-operator/verify_independent.py \
  --rust-output /tmp/aero-dynamic-observation-audit.json
~~~

The separate Python implementation independently checks the catalog, hostile mutations, synthetic comparison behavior and Rust audit output.

## Hostile cases covered

- direct RMS comparison with different temporal responses;
- incompatible units or coordinate frames;
- unadmitted source or calibration;
- required spatial mask omitted;
- unknown sample rate or incomplete PSD method;
- absent common timebase or excessive synchronization uncertainty;
- modal claim without admitted structural-mode evidence;
- promotion of the 9 kHz wind-tunnel-noise feature;
- promotion of preliminary deflection to quantitative validation;
- invented sampling rate, removed uPSP filtering/masks, or deleted measurement family.

## Standards references

- ISO 18431-1: transformations, signal categories and units for quantitative comparison of vibration/shock data. https://www.iso.org/standard/35588.html
- ISO 18431-2: time-domain windows used before Fourier-transform spectral analysis. https://www.iso.org/standard/35589.html

These are implementation references, not a claim of ISO certification. No DPW-8 PSD processing parameters are inferred from these standards.

## Claim ceiling

This crate verifies a synthetic measurement-comparison contract and reports the current source-readiness limitations. It does not validate Test Case 3, identify structural modes, establish flutter/buffet clearance, validate wing deformation, qualify Wren 2 or Wayfarer 4, support certification or authorize testing/flying.
