# WPT dual-frequency grid spectral evaluator v0

**Status:** research contract and synthetic known-truth evaluator  
**Parent:** [#3757 WPT-DUAL-FREQUENCY-GRID-001](https://github.com/Luminous-Dynamics/symtropy/issues/3757)  
**Related:** #3715 closed-loop WPT/grid stability; #3546 frequency-dependent multiport passivity; #3480 multi-frequency resonant-network isolation.

## Purpose

Preserve and qualify synchronized measurements from the grid-side current spectrum and the high-frequency resonant-current spectrum of one WPT operating episode.

The first evaluator computes two distinct metrics from frozen RMS spectral lines:

- Grid current THD: sqrt(sum of harmonic current squares above the fundamental) divided by the fundamental RMS current.
- Resonant sideband-to-carrier RMS ratio: sqrt(sum of declared sideband current squares) divided by the carrier RMS current.

This v0 does not perform FFT line extraction from raw waveforms. It checks the evidence metadata and spectral-line contract, then independently recomputes the metrics from the supplied line values.

## Synthetic known-truth fixture

The bundled fixture contains a 50 Hz grid-current spectrum and an 85 kHz resonant-current spectrum with ±50 Hz sidebands. The declared sample rate is 1 MHz with 100,000 samples, giving a 10 Hz native Fourier spacing and a 500 kHz Nyquist frequency.

Hand-calculated results:
- Grid THD = sqrt(0.5^2 + 0.3^2 + 0.2^2) / 10 = 0.0616441400 (6.1644%).
- Sideband/carrier RMS = sqrt(0.4^2 + 0.5^2) / 20 = 0.0320156212 (3.2016%).

These are synthetic evaluator values, not experimental WPT measurements or industry performance claims.

## Admission checks

- All channel and spectral-line objects are closed and unknown keys fail.
- Channel IDs are unique; each channel's frequency lines are strictly increasing.
- Every spectral frequency is positive and below Nyquist.
- Sample rate and count establish the native finite-record resolution.
- Declared effective resolution cannot claim finer resolution than the finite acquisition window.
- Effective resolution must be sufficient for the declared minimum sideband spacing.
- Grid lines must match integer harmonics of the declared grid frequency and include one non-zero fundamental.
- Resonant lines must contain one carrier and sidebands whose frequencies equal carrier plus their declared offsets.
- Grid-current and resonant-current channels must both be present.
- Synthetic evidence must carry SYNTHETIC_ONLY; non-synthetic evidence may not carry that flag.
- A cross-frequency transfer claim must declare LTP, HSS or SWITCHED_EMT analysis. Stationary LTI is not accepted as a cross-frequency conversion model.

## Limitations

- Spectral line extraction, window correction, leakage estimation and instrument uncertainty propagation are not implemented.
- THD and sideband ratios are point estimates. Expanded line uncertainties are exposed but are not combined under an unproven independence assumption.
- A sample-rate/Nyquist and finite-record-resolution check does not establish analyzer accuracy or resolve every window-dependent leakage problem.
- HSS truncation convergence and generalized-Nyquist stability are not implemented here.
- Passing this contract is not grid-code, EMC, electrical-safety, hardware or standards certification.

## Run

`cargo test --locked -p wpt-dual-frequency-grid`

CI executes this package test directly.