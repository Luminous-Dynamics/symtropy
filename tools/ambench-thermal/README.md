# NIST AM-Bench 2025-07 PCRTAM thermal-observable reference

Standalone Rust extraction/scoring utility for the CHAL-AMB2025-07-PCRTAM thermal-history challenge. The official challenge PDF is the governing observable definition.

## NIST definition implemented

Reference: AMB2025-06/07 Benchmark Measurements and Challenge Problems, §3.1 (pp. 13–14), §4.3 (p. 21):
https://www.nist.gov/system/files/documents/2025/12/18/AMB2024-06%20and%2007%20challenge%20descriptions_v3.pdf

- Thermography temperature conversion uses emissivity epsilon=0.5. This tool consumes already-converted temperatures in degC and records that upstream conversion receipt.
- TAM threshold is 1298 °C. For each pixel, linearly interpolate every threshold crossing, form all continuous intervals where T>1298 °C, and return the maximum interval duration.
- SCR uses the same selected TAM spike. On its descending branch, linearly interpolate crossings at 1260 °C and 1150 °C, then calculate 110 °C divided by elapsed seconds.
- Missing values remain absent with a reason, never converted to zero.
- Each output collection reports valid count, missing count, mean, median, sample SD and population SD, plus the full per-pixel arrays.
- Predicted vs reference arrays are compared by two-sample KS D for TAM and SCR. No KS p-value is produced because spatial pixels are not treated as independent replicates.
- Paired spatial errors are emitted only if both arrays use the identical declared uniform grid and coordinate cells; no nearest-neighbour or implicit resampling occurs.

## Metric scope correction

The PDF's Eq. 3 mean percentage-difference metric belongs to AMB2025-06-PST. It is **not** the PCRTAM score. AMB2025-07-PCRTAM specifies per-pixel TAM/SCR, summary statistics and full spatial arrays for comparison to thermography. AMB2025-06-PMPG and AMB2025-07-PMPG each have separate geometry/image metrics. This tool does not reuse those scores.

## Input evidence

Each collection binds source dataset DOI/version, whole-source SHA-256, separate grid and time-axis SHA-256 digests, thermography conversion receipt, emissivity metadata, canonical temperature/time units, algorithm version, uniform grid and per-pixel source digests. The CSV echoes these fields, grid geometry and time-axis extent so the output remains attached to the exact extraction context. Every profile uses one collection-wide strictly increasing time axis. Missing frames invalidate that pixel's extraction rather than causing interpolation across gaps.

The scorer does not parse raw camera video, convert radiance to temperature, authenticate signatures, validate the CMM/thermal measurement system, or establish the validity of the physical model.

## Synthetic tests

- known-answer single spike;
- longest multi-spike TAM selection and deterministic earliest tie break;
- same-spike SCR;
- exact threshold samples;
- missing and nonfinite temperatures;
- invalid time axis and spatial grid;
- emissivity and conversion evidence;
- empirical KS D known answer;
- paired errors only under exact coordinate alignment;
- max 1e6 output point limit;
- deterministic replay.

The synthetic fixture verifies the extraction contract, not AM-Bench winner performance or a model's physical accuracy.

## Build

    rustc --edition=2024 -D warnings --test tools/ambench-thermal/pcrt_07.rs -o /tmp/pcrt-tests
    /tmp/pcrt-tests

    rustc --edition=2024 -D warnings tools/ambench-thermal/pcrt_07.rs -o /tmp/pcrt
    /tmp/pcrt --verify
    /tmp/pcrt --csv

## Data admission and claim ceiling

Use NIST's AMB2025-06/07 calibration dataset DOI 10.18434/mds2-3707 after pinning the exact files, byte sizes, checksums, data dictionary and file-level terms. Raw datasets are not committed here. NIST's first-place AMB2025-07-PCRTAM team is from TUM/Bauhaus-Weimar, but the award page is not a numerical leaderboard or method disclosure.

This utility is an observable extraction/reference scorer. It does not simulate melt-pool flow, phase transformations, alloy thermodynamics or manufacturing yield; it is not a production qualification or machine control system.
