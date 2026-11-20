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


### E. Causal bounded linear predictor (no future source samples)

Real Co-Simulation participants often expose only the current output at a communication point. The causal method uses the current and previous committed source samples:

`Qbar_n = clamp(Q_n + 0.5 * (Q_n - Q_(n-1)), Q_min, Q_max)`

The first interval bootstraps from the current sample. The source envelope is explicit and each projection is counted; the receipt includes maximum correction. No future FMI sample is read. On the same 120-second sinusoidal oracle, 15-step endpoint error falls from 0.0101885 K (left-endpoint ZOH) to 0.000787129 K, about **12.94× lower**. It remains about 6.13× less accurate than the non-causal exact interval-average oracle at 15 steps, as expected. The frozen ladder exhibits approximately second-order global convergence on this smooth input.

The causal label refers to data access, not a universal error guarantee. The analytic test generates the discrete samples from a known sinusoid; on an unknown real FMU, predictor residual must be measured against subsequently observed source behavior. The receiver energy balance closes against the declared predicted input—not against an unobserved future source integral. Limiter activity and corrections must remain visible.

### F. Event-fenced adaptive step proposal

A deterministic policy primitive now accepts three independent normalized error signals: trajectory error, interface mismatch and physical energy residual. It uses the maximum of the three, a documented order-based step factor, and clips the proposed horizon at the earliest supplied event/clock time.

The policy distinguishes five outcomes: `Accept`, `RejectAndRetry`, `ProcessEventNow`, `ModelUncertaintyBudgetExceeded`, and `FailClosedAtMinimumStep`. It never emits a zero-length FMI doStep: an event due at the current time is an instruction to process the event. A known event may require an accepted step shorter than the configured minimum; that exception is explicit. If an unaccepted attempt would require a retry below the minimum, the policy fails closed. A past event timestamp is rejected.

Model/parameter uncertainty is a separate stop gate, not a fourth quantity that is assumed to fall when the step is shortened. If normalized uncertainty exceeds its own budget, the policy returns `ModelUncertaintyBudgetExceeded` and proposes no continuous advancement. A known event already due at the current time is processed first; subsequent continuous stepping is re-evaluated against the uncertainty budget. This keeps missing calibration or poorly constrained parameters from being disguised as a smaller numerical truncation error. The default, over-budget and event-priority cases are frozen in the JSON fixture.

This is a scheduler decision primitive, not yet an FMI master. It does not generate its own error estimates, infer future events, execute doStep, handle rollback or commit physical transfers. Those remain with the numerical estimator, FMI adapter, #6198 trace owner and #6154 physical ledger. The frozen policy cases are in the JSON fixture; eight Rust tests cover the decision boundaries. The importer-integrated work is tracked separately in [#6393 — adaptive master scheduler](https://github.com/Luminous-Dynamics/symtropy/issues/6393).


### G. Causal cumulative-energy / integrated-flux handshake

An FMU can expose a cumulative transferred-energy output in joules at each completed communication boundary. The importer derives the interval transfer from two successful observations:

`E_interval = E_cumulative(t1) - E_cumulative(t0)`

A decrease is not treated as negative heat or silently wrapped: it indicates an unobserved reset/restart and is rejected until a new counter epoch is explicitly established. The receiver consumes the declared interval energy and represents it as average power over the same interval for its state update. This method uses no future source sample and preserves the producer's reported interval integral exactly in the accounting record.

For the 120-second sinusoidal case, the cumulative-energy delta is 68,333,461.00984275 J. The receiver's average-power representation is 569,445.5084153563 W and closes its declared interval energy balance with zero residual. The endpoint temperature is 2.9899793275 K versus 3.0292389827 K for continuously varying forcing: **the interface energy is exact as declared, but the receiver's within-window trajectory remains approximate**. This separates conservation from trajectory accuracy rather than conflating them.



#### Exactly-once cumulative-energy cursor

The oracle now includes an immutable counter cursor: `(epoch_id, sequence, committed_time, cumulative_energy_J)`. Each accepted observation must be in the same explicit epoch, have exactly the next sequence number, occur at a strictly later committed time, and pass the Float64 resolution budget. A duplicate sequence, sequence gap, epoch change, repeated time, counter regression or precision-budget failure is rejected. The operation is pure: on failure the prior cursor remains unchanged. A restart must be reconciled and explicitly begin a new epoch; it cannot be inferred from a falling counter and cannot be treated as negative energy.

This makes the integrated-flux profile an exactly-once *application contract* on the observed event stream, not a guarantee of at-least-once transport or producer honesty. The master must bind each cursor advancement to its FMI API event, normalized step, and physical transfer commit barrier. Frozen cases are in `fixtures/known-answers.v0.1.json`.

#### Cumulative Float64 precision is a separate hard gate

Subtracting two finite monotone Float64 cumulative counters is not automatically an accurate interval-energy measurement. As the counter magnitude grows, the representable spacing grows too; a small interval increment can be rounded away. The oracle now returns a `CumulativeEnergyDelta` receipt with an explicit bound equal to the sum of one-ULP endpoint spacings; `integrated_flux_exchange_from_cumulative` accepts the delta only if that bound fits a caller-supplied absolute-energy error budget, and returns the receipt beside the receiver balance. A synthetic counter near 1e18 J has a 256 J step that is rejected under a 1 J budget, but can be accepted under a 1024 J budget with the bound retained in the receipt.

This is a numerical quantization bound under the assumed Float64 round-to-nearest observation model, not a sensor/model uncertainty bound and not proof of producer honesty. For long-lived large counters or tight tolerances, prefer a per-interval integrated-energy output in J, or define a higher-precision/epoch-based counter profile. A reset/restart must begin a new explicit counter epoch; it must never be interpreted as negative interval energy.

The method is causal only after the producer interval completes and only if the producer's cumulative-energy output is itself trustworthy and correctly integrated. It does not prove the source physics or importer honesty. It also does not provide a serialized checkpoint; a scalar cumulative-energy observation is a different evidence object. The FMI/SSP/API port profile is tracked in [#6407 — integrated-flux interface](https://github.com/Luminous-Dynamics/symtropy/issues/6407).

## Frozen known answers

The machine-readable inputs, decimal reference values, ZOH/interval-average/causal-predictor convergence ladders, cumulative-energy producer samples, Float64 resolution cases and adaptive step-policy cases are in `fixtures/known-answers.v0.1.json`. Rust tests verify frozen answers, partition invariance, exact interval-average source energy, limiter diagnostics, first- and second-order convergence, event fences, minimum-step refusal, uncertainty gating, counter epoch/sequence atomicity and cumulative-counter precision limits.

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
