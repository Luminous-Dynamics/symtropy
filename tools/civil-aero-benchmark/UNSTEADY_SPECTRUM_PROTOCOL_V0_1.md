# Unsteady buffet spectrum and stationarity protocol v0.1

## Scope and evidence ceiling

This protocol defines an unsteady-signal processing lane separate from steady RANS admission. It covers a time history's stationarity diagnostics, Welch one-sided PSD, integrated band power, dominant frequency and normalized spectral-shape comparison. It does not solve the Navier–Stokes equations, validate a turbulence model, establish buffet onset by itself, or validate aeroelastic flutter.

Synthetic known-tone tests validate only spectral arithmetic and data handling. Experimental validation requires lawful, exact-revision data with correct sensor mapping, units, tunnel conditions, uncertainty and processing settings.

## Welch convention in v0.1

The implementation requires an explicit sample interval, power-of-two segment length and overlap. It applies per-segment mean removal, a periodic Hann window and a radix-2 FFT. The one-sided PSD scaling is:

PSD[k] = dt * abs(FFT(window * detrended_signal)[k])^2 / sum(window^2)

Interior positive-frequency bins are doubled; DC and Nyquist are not. Segment PSDs are averaged. Frequency-bin spacing is df = fs/N. Integrating the discrete one-sided PSD with the same df gives the mean-square estimate for this convention.

The report records sample interval, sampling rate, segment length, overlap, stride, number of Welch segments, window, detrending convention, frequency bins and PSD units. The caller must keep units consistent (for pressure coefficient, PSD is per Hz).

## Stationarity

The first diagnostic partitions the trace into a caller-selected number of contiguous windows and reports minimum/maximum means, absolute mean drift, minimum/maximum RMS and fractional RMS drift. It does not automatically discard transients. The analyst must specify and retain any transient-removal interval before invoking the analyzer. Stationarity thresholds must be predeclared and justified for the observable; the implementation reports metrics but does not impose one universal physics threshold.

## Known-answer validation

The synthetic trace is a 32 Hz sinusoid with amplitude 2, offset 1.25, 512 Hz sampling and 1024 samples. The Welch configuration uses 256 samples per segment and 128-sample overlap, producing seven segments and 129 one-sided bins. Expected values are:
- dominant peak frequency: 32 Hz;
- integrated mean-square: 2;
- time-domain RMS after global mean removal: sqrt(2);
- 30–34 Hz band power: 2 (within the predeclared numeric tolerance);
- zero window-to-window mean and RMS drift.

Rust unit tests also require detection of an amplitude step, different-frequency spectral shape, invalid frequency-bin comparison, non-finite input, invalid overlap and insufficient traces. An independent standard-library Python implementation computes the DFT directly and checks every frequency bin and PSD value.

## DPW8 unsteady validation ladder

1. **V1b OAT15A extended polar:** use unsteady settings for the official alpha set and grid levels; report time-step, sampling, stationarity and spectral convergence separately from V1a steady convergence.
2. **CRM Buffet Case 2:** bind the correct deformed rigid geometry, wind-tunnel conditions, committee grid and sensor-coordinate files. Compare time-averaged/RMS Cp, sensor time histories, PSD shape, peak frequency and band power.
3. **CRM Buffet Case 3:** add coupled structural degrees of freedom, modal frequencies, phase/amplitude, pressure/strain coherence, structural work/energy balance and coupled time-integration convergence. A rigid-wing Case 2 result is not Case 3 evidence.

For time-step refinement, use at least three suitable dt values and report dt, sampling Nyquist, samples per dominant cycle, temporal refinement ratio, observed temporal order where justified, and changes in each declared spectral metric. Spatial and temporal refinement must be reported separately.

For spectral comparisons:
- use the same sensor ID/location, axis convention, units and nondimensionalization;
- disclose segment length, overlap, window, detrending, sampling interval and transient removal;
- compare time-domain RMS with integrated PSD RMS;
- compare dominant frequency and Strouhal number with explicit reference velocity and length;
- compare integrated band power and normalized spectral shape;
- use coherence and phase where simultaneous channels and timing alignment are available;
- report uncertainty from independent blocks or a justified block-bootstrap procedure; overlapping Welch segments are not independent replicates.

## Reference procedure example

The JAXA DPW8 CRM Case 2 public page documents an experimental PSD procedure using 20 microseconds sampling interval, 499,712 samples, 8,192 samples per block, 121 averages, 50% overlap, one-sided PSD and normalization whose integral corresponds to time-domain RMS. These are reference-specific settings, not universal defaults.

## References

- Official DPW8 test-case matrix: https://www.aiaa-dpw.org/TestCases/testcases.html
- AIAA Buffet Working Group: https://aiaa-dpw.larc.nasa.gov/WorkingGroups/Group3/group3.html
- JAXA CRM Case 2 experimental data and PSD procedure: https://cfdws.chofu.jaxa.jp/apc/dpw/upc.html
- DPW8 postprocessing templates: https://www.aiaa-dpw.org/postprocessing.html
