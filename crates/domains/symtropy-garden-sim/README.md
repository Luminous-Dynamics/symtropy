# Symtropy Garden Infrastructure Scenario Kernel

This isolated Rust crate provides a deterministic accounting checker for one resource balance. It is a starter kernel for the proposed garden-infrastructure scenario described in [the garden-to-facility contract](https://github.com/Luminous-Dynamics/symtropy/pull/1630).

## Run

```sh
cargo fmt --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml -- --check
cargo test --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml
cargo clippy --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml --all-targets -- -D warnings
```

## Claim ceiling

The checker independently recomputes a declared interval balance from supplied raw quantities. It does not simulate fluid flow, infer water quality, predict crop yield, identify plants, model medicine stability, or prove physical validity. Inputs can still be wrong or fabricated; source references are not fetched or authenticated.

The next integration step is to bind this small contract into Symtropy's existing replay/scenario substrate after confirming workspace ownership and supported physics-model capabilities.
