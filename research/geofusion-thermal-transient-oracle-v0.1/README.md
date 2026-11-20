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

The integral of the sinusoidal input, sinusoidal temperature terms and exponential transient is evaluated analytically. A master with zero-order hold can then be measured against the continuous-input reference without using the FMU's own reported subtotals as its oracle.

## Frozen known answers

The machine-readable inputs and decimal reference values are in `fixtures/known-answers.v0.1.json`. Rust tests verify the same frozen values and the independent metamorphic property that splitting a piecewise-constant interval into subintervals produces the same endpoint and integrated balance.

## Run

```sh
cargo fmt --manifest-path research/geofusion-thermal-transient-oracle/Cargo.toml -- --check
cargo test --manifest-path research/geofusion-thermal-transient-oracle/Cargo.toml --all-targets
cargo clippy --manifest-path research/geofusion-thermal-transient-oracle/Cargo.toml --all-targets -- -D warnings
```

## Benchmark composition

This oracle is intended to feed #6224:
- OMSimulator `oms_ma`, `oms_mav`, `oms_mav2`: direct master comparisons.
- FMPy: importer/interop smoke runner, not a like-for-like master.
- preCICE and SUNDIALS/ARKODE: adjacent coupling/integrator tracks only, with equivalent participant equations and scope.

Use identical FMU revisions, parameters, initial conditions, input waveform/hold policy, event schedule, output times, tolerances and compute budgets. FMI 2 and FMI 3 capabilities remain separate lanes.

## Claim ceiling

A pass qualifies the analytic implementation for the declared synthetic lumped thermal family. It does not validate physical exchanger hardware, a geothermal well or reservoir, any real installation, economics or safety. No comparative superiority or SOTA claim can be made until the matched transient campaign is executed and independently reproduced.
