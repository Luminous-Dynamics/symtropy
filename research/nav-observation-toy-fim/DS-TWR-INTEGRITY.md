# DS-TWR interval integrity baseline v0.1

## Equation

MILUV uses a variation of double-sided two-way ranging (DS-TWR). Its paper prints the range equation:

`y = c/2 * ((R2-T1) - ((R3-R2)/(T3-T2)) * (T2-R1))`

This module names the four duration terms A, B, C and D in that order. Each term is supplied as a duration in seconds with an independently justified hard absolute error bound.

## Interval propagation

For each positive duration, the input interval is `[centre - abs_error, centre + abs_error]`. The clock-rate ratio interval is bounded by:

`[B_low / C_high, B_high / C_low]`

The ratio is multiplied by the positive D interval, then subtracted from the A interval. The time-of-flight lower bound is clipped at zero because a physical range is nonnegative; a transaction whose upper time-of-flight bound is negative is rejected.

The result carries nominal, lower and upper ranges and clock-ratio bounds. It is deterministic interval arithmetic, not a Gaussian standard deviation or probability.

## Critical adapter boundary

The function intentionally does **not** map raw CSV `tx1/rx1/tx2/rx2/tx3/rx3` columns into A/B/C/D. The protocol describes timestamps held in separate transceiver clocks, and a source adapter must verify the exact event-to-clock mapping, counter rollover/unwrapping, conversion from DW1000 ticks to seconds and the protocol revision before constructing these four durations. A guessed mapping would make the math look rigorous while applying the wrong clocks.

Similarly, the input error bounds do not currently derive from `skew1/skew3`, CIR or antenna-delay calibration. They must be qualified separately. The interval method can be extended once a source-verified timestamp adapter exists.

## Research comparison

Compare this interval baseline with the authors' precomputed `range`, `range_raw`, `std`, `bias`, and `gt_range` outputs, but keep `gt_range`/bias strictly evaluator-only. Evaluate coverage of the interval against independent Vicon geometry, interval width, false acceptance outside a fixed range-error threshold and performance under anchor/trajectory/obstacle splits.

Do not tune timestamp error bounds against the same Vicon truth used to report range-interval coverage.

## Validation

Unit tests verify the nominal equation, monotonic interval widening, nominal ratio containment, fail-closed denominator/physical range handling and non-finite/negative inputs. The demo uses synthetic durations and bounds; it does not parse or score MILUV data.