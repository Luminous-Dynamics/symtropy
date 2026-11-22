# Mobility Downstream Fence Contract v0.1

Dependency-free Rust reference contract for checking per-layer evidence across a declared command-holding path.

## Scope

The contract checks:
- one exact receipt per declared command-holding layer;
- unique layer identity and contiguous path order;
- exact stack, inventory, layer, logical-effect and old-attempt bindings;
- old/new boot and controller epoch bindings;
- sequence-generation continuity;
- accepted/retired high-water equality and zero outstanding work;
- an admissible layer disposition (`quiescent`, `cancelled`, `drained`);
- declared issuer identity and a separately named verifier failure domain.

Run:

```sh
cargo fmt --manifest-path research/mobility-downstream-fence/Cargo.toml -- --check
cargo test --manifest-path research/mobility-downstream-fence/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-downstream-fence/Cargo.toml --all-targets --locked -- -D warnings
```

## Non-claims

The result is only `AllDeclaredLayersHaveCurrentQuiescenceEvidence` for the exact inventory and supplied receipt packet. It is not:
- proof that inventory enumeration is complete;
- cryptographic authentication of any receipt or commitment;
- proof of actual cancellation linearizability or device/firmware behavior;
- proof that a physical effect did not already happen;
- a physical-effect observer or NoEffect classifier;
- proof of true failure-domain independence;
- authority to issue a new action or call a native sink.

The external inventory owner and an independent verifier must qualify the command path. Missing or unknown downstream dependencies stay fail-closed. A successful software test only checks the reference contract over synthetic evidence.
