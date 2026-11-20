# GeoFusion thermal transient analytic oracle v0.1

**Tracking:** [#6343 — GEOFUSION-THERMAL-TRANSIENT-ORACLE-001](https://github.com/Luminous-Dynamics/symtropy/issues/6343).  
**Status:** standalone closed-form reference kernel; synthetic mathematics only.

## Why this is separate from the FMU smoke

The constant-power FMUs in #6337 prove initial package/API plumbing and elementary energy allocation. They cannot distinguish co-simulation masters numerically. This crate supplies independently evaluable dynamic trajectories and integrated energies for a later matched tournament in #6224. It implements no FMI master, importer, physical solver or runtime coupling engine.

## Model

A lumped thermal buffer with excess temperature `theta=T-T_ambient` obeys:

`C d(theta)/dt = (1-alpha) Q_source(t) - UA theta`

`Q_service = alpha Q_source`  
`Q_loss = UA theta`  
`Q_storage = C d(theta)/dt`

The instantaneous first-law identity is `Q_source = Q_service + Q_loss + Q_storage`. The oracle reports interval-integrated energy in joules; it never sums power samples as if they were energy.

## Two closed-form tiers

### A. Piecewise-constant source

For constant `Q` over `dt`, the exact endpoint is:

`theta_inf = (1-alpha) Q / UA`  
`tau = C / UA`  
`theta_end = theta_inf + (theta_start-theta_inf) exp(-dt/tau)`

The exact loss integral is evaluated using `expm1` to reduce cancellation. Service energy, input energy and storage change are computed independently and the residual is recomputed.

### B. Sinusoidal source

For `Q(t)=Q0+A sin(omega t)`, the library evaluates the closed-form response:

`theta(t)=theta_c+a sin(omega t)+b cos(omega t)+K exp(-k t)`

where `k=UA/C`, `theta_c=(1-alpha)Q0/UA`, `a=((1-alpha)A/C) k/(k^2+omega^2)`, `b=-((1-alpha)A/C) omega/(k^2+omega^2)`, and `K=theta(0)-theta_c-b`.

The integral of the sinusoidal input, sinusoidal temperature terms and exponential transient is evaluated analytically. The implementation uses midpoint/sinc forms for trigonometric interval integrals to avoid cancellation for narrow communication windows, and `expm1` for the exponential transient integral.


### C. Exact interval-average flux exchange

The reference also computes the exact average of the sinusoidal source over each communication interval:

`Q_bar[k] = (1 / dt) * integral(Q_source(t), t_k..t_(k+1))`

The receiver holds this average constant during the interval and applies the exact piecewise-constant state transition. Therefore the energy supplied during each interval is exact for the declared sinusoidal source, even though the thermal trajectory remains an approximation to continuous forcing.

This is a conservative *oracle/method reference*, not a universally causal master policy. It requires a known/forecast waveform or a source FMU that can publish an integrated interval flux. A generic importer cannot infer the future interval integral from a single point sample. The frozen fixture compares it with left-endpoint zero-order hold over the same 15–960 communication grids: at 15 steps, endpoint error falls from about 0.01019 K to 0.0001283 K (about 79.4× lower), and the observed global convergence is approximately second order. These are oracle calculations, not measured OMSimulator or commercial-tool results.

### D. Zero-order-hold communication ladder

The crate also samples the same sinusoidal source at each communication interval's left endpoint and holds that input constant over the interval. It solves each held interval exactly, then compares the final temperature against the continuous sinusoidal reference. A frozen 15-to-960-step ladder in the JSON fixture shows the expected approximately first-order global convergence of left-endpoint zero-order hold. This is a mathematical discretization study, not an OMSimulator or commercial-master performance result.

## Frozen known answers

The machine-readable inputs, decimal reference values and both communication-grid ladders are in `fixtures/known-answers.v0.1.json`. Rust tests verify the frozen cases, partition invariance for constant inputs, zero-amplitude reduction, exact interval-average source energy, monotonic grid refinement, and first- versus second-order convergence.

## Run

```sh
cargo fmt --manifest-path research/geofusion-thermal-transient-oracle-v0.1/Cargo.toml -- --check
cargo test --manifest-path research/geofusion-thermal-transient-oracle-v0.1/Cargo.toml --all-targets
cargo clippy --manifest-path research/geofusion-thermal-transient-oracle-v0.1/Cargo.toml --all-targets -- -D warnings
```

## Benchmark composition

This oracle is intended to feed #6224:
- OMSimulator `oms_ma`, `oms_mav`, `oms_mav2`: direct master comparisons.
- FMPy: importer/interop smoke runner, not a like-for-like master.
- preCICE and SUNDIALS/ARKODE: adjacent coupling/integrator tracks only, with equivalent participant equations and scope.

Use identical FMU revisions, parameters, initial conditions, input waveform/hold policy, event schedule, output times, tolerances and compute budgets. FMI 2 and FMI 3 capabilities remain separate lanes.

## Claim ceiling

A pass qualifies the analytic implementation for the declared synthetic lumped thermal family. It does not validate physical exchanger hardware, a geothermal well or reservoir, any real installation, economics or safety. No comparative superiority or SOTA claim can be made until the matched transient campaign is executed and independently reproduced.
