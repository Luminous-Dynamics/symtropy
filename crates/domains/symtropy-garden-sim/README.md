# Symtropy Garden Infrastructure Scenario Kernel

This isolated Rust crate provides an exact integer accounting checker for one resource balance. It is a starter kernel for the proposed garden-infrastructure scenario described in [the garden-to-facility contract](https://github.com/Luminous-Dynamics/symtropy/pull/1630).

## Resource units

- Water: integer millilitres
- Electrical energy: integer watt-hours
- Material: integer grams

Every field in one balance must use the unit fixed by its resource kind. The checker does not perform unit conversion. Flow, addition, loss and tolerance fields are non-negative unsigned integers; the storage delta is signed. Expected balance is recomputed with wide integer arithmetic and the absolute error uses overflow-safe integer difference.

## Run

```sh
cargo fmt --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml -- --check
cargo test --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml
cargo clippy --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml --all-targets -- -D warnings
```

## Claim ceiling

The checker independently recomputes a declared interval balance from supplied quantities. It does not simulate fluid flow, infer water quality, predict crop yield, identify plants, model medicine stability, or prove physical validity. Inputs can still be wrong or fabricated; source references are not fetched or authenticated. A consistent accounting result is not proof that the physical model or external observation is correct.

The next integration step is to bind this contract into Symtropy's existing replay/scenario substrate after confirming workspace ownership and supported physics-model capabilities.
