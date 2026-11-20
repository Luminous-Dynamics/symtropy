# CMNI reference kernel

This is a **research-only numerical prototype** for a homogeneous screened D–D thick-target yield integral.

It deliberately does not:
- implement first-principles nuclear physics or a validated nuclear-data library;
- supply a real astrophysical S-factor or material stopping-power dataset;
- represent spatially heterogeneous defects, implantation or hydrogen transport;
- reproduce any published experiment;
- estimate power gain, calorimetric excess heat, or net energy.

The caller must inject sourced, versioned S-factor and stopping-power functions, target density, screening model, and uncertainty metadata. Current tests use synthetic constants only to verify numerical behavior and input rejection.

Run with:

```sh
cargo test --manifest-path research/cmni/reference-kernel/Cargo.toml
```

The kernel is intentionally outside the production workspace pending independent physics review, source-data reconstruction, and numerical verification. See [the physical twin design](../../docs/research/cmni/PHYSICS_TWIN.md).
