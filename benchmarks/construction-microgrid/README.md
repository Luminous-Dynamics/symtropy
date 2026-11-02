# Construction microgrid reference evaluator v0.1

Dependency-free, synthetic-first supply-versus-load accounting evaluator for [CONSTRUCTION-MICROGRID-001 #3314](https://github.com/Luminous-Dynamics/symtropy/issues/3314).

## Run

```sh
cargo test --locked --manifest-path benchmarks/construction-microgrid/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/construction-microgrid/Cargo.toml --all-targets -- -D warnings
```

## V0 contract

- Compares a frozen set of interval demands against declared per-interval source availability and power envelopes.
- Source types are grid import, renewable generation, conventional generation and external firm supply.
- External firm supply is an external interface only; an SMR candidate has no reactor model, licence logic, safety system or operating authority here.
- Source priority is explicit input, not an optimized dispatch policy; the current baseline greedily serves critical load first, then flexible load, and charges storage only after demand is met.
- Missing supply availability is unknown; it is not zero.
- Optional variable cost/emissions remain unknown if a used source lacks an input.
- Critical demand is served before flexible demand.
- Battery charge/discharge uses integer Wh and parts-per-million efficiencies, enforces reserve and power/energy limits, and never charges and discharges in the same interval.
- Per-interval conservation checks source availability = source-to-load + battery charge input + curtailment; demand = source-to-load + battery-to-load + unserved; stored energy reconciles with charge/discharge losses.
- The delivered-load energy intensity retains the exact numerator (Wh) and denominator (accepted output units); it is not truncated to an integer decimal ratio. It is not the upstream generation-energy intensity.
- Battery initial state of charge is an explicit pre-horizon inventory. A horizon that draws down initial SOC must account for the initial charge provenance/cost outside this v0 evaluator before making a full lifecycle cost claim.

## Not modelled in v0

Ramp-rate, minimum stable output, sub-interval motor-start waveforms, voltage/frequency stability, harmonics, protection coordination, black-start sequencing, thermal/hydrogen/nuclear plant physics, battery degradation, financing/capital recovery and life-cycle cost are not simulated. They must be added as versioned interfaces or specialist engineering studies before a real design decision.

All input values in the tests are synthetic. Variable cost and emissions use conservative per-source/per-interval rounding to integer microcurrency/grams; these are screening totals, not settlement-grade billing or a lifecycle assessment. A full energy balance is not evidence of safe power quality, electrical approval, SMR licensing or dispatch authority.
