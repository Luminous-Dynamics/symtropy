# CNC multi-rate telemetry reference

This first tranche qualifies measurement admission and deterministic features only. It does not infer tool wear, remaining life, machine health, part quality, or maintenance actions.

## Contract

- Each sensor channel retains its own sampling rate, unit, clock-domain identity, calibration identity, raw timestamps and supplied source SHA-256.
- Clock conversion uses an explicit affine map: reference time = local time + offset + drift correction around a declared origin. Missing mappings and uncertainty above 100 μs are INCONCLUSIVE.
- Windows are reference-clock half-open intervals [start,end). No interpolation, forward fill or zero fill is performed.
- Timestamp intervals must be monotone and remain within 10% of an integer multiple of the declared sample period. Integer multiples greater than one are explicit dropped-sample gaps.
- v0.1 supports only vibration in m/s² and current in A; other units fail closed rather than being silently converted.
- Calibration must remain valid through the entire window.
- Per-channel features: mean, RMS, peak-to-peak, crest factor, unbiased sample variance and population excess kurtosis.
- Cross-channel Pearson correlation is available only for distinct vibration/current sensors on the same reference clock, with combined clock uncertainty <=100 μs, identical nominal sample rates and at least three exact co-timed samples. No nearest-neighbour matching or interpolation is allowed. Unequal-rate fusion returns INCONCLUSIVE until a versioned anti-alias resampling/filter and phase-delay contract is implemented. This avoids silently down-sampling 25 kHz vibration into 500 Hz current and producing an aliased but plausible correlation. Even exact-rate Pearson is a descriptive diagnostic, not a causal or physical coupling model.

The source digest is supplied by the upstream admission layer and carried through the feature row; this program does not pretend to compute a digest over unknown raw source bytes. The workflow separately SHA-256 hashes the checked-in source and exact output bytes.

## Build and run

    rustc --edition=2024 -D warnings --test tools/machining-telemetry/telemetry_reference.rs -o /tmp/telemetry-tests
    /tmp/telemetry-tests
    rustc --edition=2024 -D warnings tools/machining-telemetry/telemetry_reference.rs -o /tmp/telemetry-reference
    /tmp/telemetry-reference --verify
    /tmp/telemetry-reference --csv

## Evidence boundary

Synthetic fixtures prove deterministic mechanics only. Dataset admission, source terms, calibration quality, actual machine timestamps and predictive validity require separate evidence. In particular, Piecuch Figshare data remain blocked until file-level licence metadata is verified; MATWI has CC BY-SA 4.0 obligations; NASA Milling Wear metadata identifies US Government Works terms.
