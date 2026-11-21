# Standalone CI validation

The base branch supplies .github/workflows/navigation-research-rust.yml. It is deliberately separate from the main Symtropy workspace because this crate is isolated from production membership.

The targeted workflow runs:

```sh
cargo test --all-targets --manifest-path research/nav-observation-toy-fim/Cargo.toml
```

For the transition validator, the same workflow runs:

```sh
cargo test --all-targets --manifest-path research/nav-transition-validator/Cargo.toml
```

The correlation-aware source is a module imported by src/bin/correlation_demo.rs; `--all-targets` compiles and tests that binary target as well as the library. The broad repository workspace job does not, by itself, prove that an excluded research crate was built.

No test result is claimed in this document. The exact-head job must complete successfully before treating Rust compilation or unit tests as executed evidence.
