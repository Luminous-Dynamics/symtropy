# Construction campaign power envelope v0.1

Dependency-free Rust reference evaluator for [Symtropy #3336](https://github.com/Luminous-Dynamics/symtropy/issues/3336).

## Run

```sh
cargo test --locked --manifest-path benchmarks/construction-power-envelope/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/construction-power-envelope/Cargo.toml --all-targets -- -D warnings
```

## Model

- Piecewise-constant real-power load intervals in watts and seconds.
- Critical and flexible load separated; firm supply serves critical load first, then flexible load.
- Optional battery represented by discharge power (W), usable stored energy (W·s), minimum reserve, and discharge efficiency.
- The battery is dispatched greedily against each interval's deficit; no future-information optimizer is used.
- Exact energy integration uses integer watt-seconds; no floating-point energy rounding.
- Supply intervals are explicit inputs and can represent grid outage, derating or an externally declared supply availability profile.

## Explicit non-claims

- No generator/reactor dynamics, power-flow, short-circuit, voltage/frequency, harmonics, protection, feeder thermal limits, motor-drive transients, reactive power or power-quality solver.
- No TBM/dredger load curve is inferred from nameplate/installed ratings.
- The 20 MWe microreactor / 22 MW TBM and 44.18 MW dredger fixtures are arithmetic full-nameplate capacity controls only, not actual duty-cycle/energy forecasts.
- The battery dispatch rule is a transparent baseline, not an optimal dispatch or certified BESS controller.
- SMR candidate supply must be externally licensed/vendor/operator-authorized interface data. No nuclear design or control model exists here.

All tests are synthetic. Real sizing requires time-resolved supplier loads, measured site meter data, licensed electrical studies, grid-operator requirements, qualified engineering and competent approvals.
