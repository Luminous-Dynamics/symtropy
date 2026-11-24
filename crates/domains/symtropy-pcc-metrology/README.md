# Symtropy PCC Metrology

A small, Rust-first assurance crate for calibrated, uncertainty-qualified point-of-common-coupling (PCC) power traces.

## Scope

The crate is the measurement gate upstream of workload-flexibility, grid-response and adequacy evaluators. It does not read physical meters, validate a calibration laboratory, dispatch plant, model the grid or certify a market product.

The v0.1 evaluator:
- applies a pinned linear gain/offset model to each raw active-power reading;
- propagates raw-reading, gain and offset standard uncertainties using a declared 3×3 covariance/correlation model;
- rejects a calibration correlation matrix that is not positive semidefinite;
- computes paired baseline-minus-candidate response with an explicit baseline/candidate error correlation;
- computes a caller-defined expanded-uncertainty interval and fails when the lower response bound or relative-uncertainty requirement is not met;
- checks calibration validity, operating range, trace quality, evidence identity, pair timestamp equality and contiguous interval chronology;
- integrates signed response energy using each interval's actual duration, preserving rebound as negative response;
- abstains for clipped, saturated, missing, stale, reset, unsupported or out-of-range readings.

## Mathematical contract

For corrected power:

`P = (z - o) g`

where `z` is raw active power, `o` is the calibration offset and `g` is the gain.

First-order uncertainty propagation uses `u(P)^2 = J Σ Jᵀ` with the full declared covariance terms for `z`, `o` and `g`. It does not silently assume these calibration inputs are independent.

For paired baseline and candidate readings:

`R = P_baseline - P_candidate`

`u(R)^2 = u(P_baseline)^2 + u(P_candidate)^2 - 2 ρ u(P_baseline) u(P_candidate)`

The correlation `ρ` is explicit evidence. It is never inferred from shared meter IDs.

The lower response bound is `R - k u(R)`, where the caller supplies `k`. The crate records that multiplier and does not claim a universal confidence level. Nonlinear/non-Gaussian models require distribution propagation and are outside this linearized v0.1 implementation.

Energy is integrated from corrected interval-average power using each interval's actual duration. Negative reduction is preserved as rebound. The sum of interval lower-bound energies is **not** a joint confidence bound unless cross-interval dependence is separately qualified.

## Run

```sh
cargo test -p symtropy-pcc-metrology
cargo clippy -p symtropy-pcc-metrology --all-targets -- -D warnings
```

The package is an independent workspace member to ensure its Rust tests actually compile. It does not depend on the currently unregistered symtropy-energy-systems source directory.

## Clock-time uncertainty

V0.2 extends time alignment beyond a clock-offset pass/fail threshold. When either trace has nonzero time uncertainty, the caller must provide externally validated conservative maximum absolute power-slew bounds for both traces across the complete event window. The evaluator adds a deterministic timing-induced power bound:

`U_time = S_baseline × u_t,baseline + S_candidate × u_t,candidate`

where `S` is the declared maximum absolute power slew in W/s and `u_t` is the clock-time uncertainty in seconds. This bound is added to the expanded electrical measurement-uncertainty allowance rather than combined by Gaussian root-sum-square. The conservative response lower bound therefore includes both electrical measurement uncertainty and worst-case timing sensitivity.

Nonzero clock uncertainty without both slew bounds is an abstention. A zero bound must itself be justified by the upstream evidence; the evaluator does not infer that a trace is flat. These bounds are caller-provided, not estimated from the same finite sample window, so the evaluator does not silently assume that observed sampled ramps bound unobserved sub-interval transients.
