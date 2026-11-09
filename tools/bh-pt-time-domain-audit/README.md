# Independent Pöschl–Teller time-domain audit

This crate is a direct, independent time-domain solver for

(d_t^2 - d_x^2 + V(x)) u = S, with V(x) = lambda / width^2 * sech^2(x/width).

It uses a second-order centered spatial operator and classical RK4 in time (method of lines). It does not call the Jost, special-function, frequency-domain Green, contour, or residue code in the complex-special-function crate.

## Boundary and source semantics

- Zero initial field and velocity.
- A smooth compactly supported space-time bump source; no Gaussian tails outside the declared source support.
- Fixed zero Dirichlet boundaries on a wide finite interval.
- No absorbing-boundary approximation is claimed. Each run reports the earliest source-to-boundary-to-observer return time. Compare only before this time.
- The observer value is linearly interpolated from the spatial grid.

## Diagnostics

- Discrete energy of the semidiscrete PDE.
- Source work integrated with the RK4 quadrature.
- Relative energy/work balance residual.
- Reflection-free boundary time.
- Observer waveform sampled at a pinned output interval.

The free-wave test compares the PDE output with the independent 1+1D retarded Green integral u=(1/2) integral(S dx dt) after the whole compact source pulse has reached the observer.

## Run

```sh
cargo test --manifest-path tools/bh-pt-time-domain-audit/Cargo.toml --all-targets
cargo run --release --manifest-path tools/bh-pt-time-domain-audit/Cargo.toml -- --output pt-waveform.csv
```

## Limitations

This is a pilot direct solver for a prescribed 1D scalar master equation. It is not hyperboloidal, does not model black-hole horizon/infinity compactification, and does not solve Teukolsky/Regge–Wheeler/Zerilli equations. Its finite-difference waveform is an independent falsification reference for the Pöschl–Teller Green-kernel work, not a replacement for issue #4052.
