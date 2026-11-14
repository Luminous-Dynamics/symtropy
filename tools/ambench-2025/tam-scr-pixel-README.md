# AMB2025-07 pixel-level TAM/SCR operator reference

Standalone Rust reference that derives NIST-style TAM and solid cooling rate from an **already calibrated** per-pixel temperature/time trace. It does not convert raw radiance, correct emissivity, remove spatter or validate the camera clock.

## NIST measurement operator

The AMB2025-06/07 detailed challenge description (last updated 2025-12-12), §3.1:
- assumed IN718 solidus = 1260 °C;
- liquidus = 1336 °C;
- midpoint = 1298 °C;
- TAM = maximum continuous time above 1298 °C in one pixel's temperature history;
- SCR = cooling over 110 °C from solidus 1260 °C to 1150 °C on the same peak, with linear interpolation.

TAM entry/exit crossings are linearly interpolated. This implementation selects the longest fully bracketed interval and resolves exact ties to the earliest interval. It does not clip a hot interval that runs into the start or end of the trace. No crossing is not a zero measurement; it is MISSING or INCONCLUSIVE.

The NIST text prints Δt=t1260−t1150, which has a negative sign if timestamps progress forward during cooling. The executable contract requires the physical positive elapsed time t1150−t1260. This discrepancy is recorded and should be clarified with the NIST measurement team; the operator never takes an absolute value to hide it.

## Emissivity / calibration source discrepancy

The detailed challenge description §3.1 states effective emissivity ε≈0.5 from earlier AMB2022-03 measurements, and §3.1.1 says to convert video using ε=0.5. The later AMB2025-06/07 challenge-results document §2.3 says single-line laser tracks gave an average effective emissivity ε=0.642 for converting radiant temperature to emissivity-corrected temperature.

These are two official published statements. This kernel does not select between them: its input is a temperature-time series with explicit transform/calibration ID and digest. Raw thermography must not be admitted until the applicable transformation and dataset/version are verified. The NIST results document also states TAM<0.5 ms and SCR>1e6 °C/s are excluded from aggregate distributions; those distribution filters are downstream in the #3003 scorer, not inside this pixel operator.

## Algorithm

1. Validate trace identities, source SHA-256 references, finite temperatures/timestamps, strictly increasing timestamps and contiguous frame indexes.
2. Find all maximal runs with T>1298 °C.
3. Interpolate both threshold crossings for each run using the actual time interval.
4. Select the longest complete run. An unbounded edge run makes TAM INCONCLUSIVE because its full duration is unknown.
5. Find the maximum measured temperature within the selected TAM run.
6. From that same peak, find the first downward 1260 °C crossing and then the subsequent downward 1150 °C crossing.
7. Reject SCR if the temperature reheats above solidus before reaching 1150 °C, if a crossing is absent, or if the physical elapsed time is nonpositive.
8. Report per-metric status, reason, crossings, source and transform hashes, and deterministic trace/operator digests.

No smoothing, despiking, gap-filling, spatter removal, or interpolation across missing frames is performed.

## Build and run

    rustc --edition=2024 -D warnings --test tools/ambench-2025/tam_scr_pixel_operator.rs -o /tmp/amb07-pixel-tests
    /tmp/amb07-pixel-tests

    rustc --edition=2024 -D warnings tools/ambench-2025/tam_scr_pixel_operator.rs -o /tmp/amb07-pixel
    /tmp/amb07-pixel --verify
    /tmp/amb07-pixel --reference

## Relationship to the summary scorer

The pixel operator emits TAM seconds and SCR °C/s with missing/reason codes. The separate #3003 scorer applies the NIST distribution-level filters and aggregates the samples across all three repeat pads per group. This preserves an auditable measurement-operator boundary between pixel physics and distribution summaries.

## Claim ceiling

This is a synthetic measurement-operator reference, not a NIST data reproduction. It does not prove the selected emissivity, radiance calibration, spatter removal, camera synchronization, material temperature or model accuracy. The only admitted input here is an already-calibrated temperature-time trace.
