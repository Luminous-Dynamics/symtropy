# Multiaxial HCF Reference

Dependency-free Rust 1.96 reference for a bounded, stress-based Findley critical-plane parameter on one stabilized, sampled HCF cycle.

## Why this slice exists

The FABEST HCF challenge uses 42CrMo4+QT plain and notched hollow cylindrical specimens under combined axial/torsional loading, including in-phase and 90-degree out-of-phase histories with constant internal pressure. A scalar equivalent-stress reduction before examining the stress path can discard non-proportionality.

This reference accepts a local six-component symmetric stress-tensor history, in MPa, and evaluates a declared Findley critical-plane parameter:

    P_F = tau_a + k * sigma_n,max

For each candidate material plane:
- traction is resolved from the full stress tensor;
- normal stress is computed on the plane;
- the in-plane shear-traction path is preserved as 2D points;
- the maximum shear range over in-plane directions is calculated as half the diameter of that path;
- the Findley parameter is maximized over a declared hemisphere grid.

The critical plane reports its local shear-path shape index; search diagnostics separately report the maximum shape index over all searched planes and the plane where it occurs. This avoids confusing a line-like critical plane with proportionality of the entire multiaxial stress path. The shape index is diagnostic only, not another fatigue-life law.

## Important boundaries

- One stabilized closed cycle only. Arbitrary variable-amplitude tensor-path cycle counting is not implemented.
- The phase samples must cover a full cycle with a bounded maximum circular phase gap.
- The plane search is discretized. The report carries the polar/azimuth resolution and number of evaluated planes. Refinement studies are required before using it for a benchmark result.
- The normal-stress sensitivity k is a material-calibration parameter. The reference includes a bounded two-point calibration function that derives k and the Findley parameter limit from fully reversed axial and torsional fatigue-strength values at the same material grade and target cycle count:
  
    r = sigma_w / tau_w
    k = (2 - r) / (2 * sqrt(r - 1))
    f = sigma_w / 2 * (k + sqrt(1 + k*k))

The function accepts only the positive-k domain 1 < r <= 2, rejects mismatched grade/cycle count and refuses predicted calibration evidence. It does not fit S-N curves or create fatigue-strength values. The material values must come from permitted calibration data; no universal or calibrated 42CrMo4+QT value is embedded.

The Findley parameter can be converted into an equivalent fully reversed axial stress amplitude:

    sigma_a,eq = 2 * P_F / (k + sqrt(1 + k*k))

That equivalent stress can be passed to a separately qualified uniaxial S-N reference (such as the existing Symtropy #4860 owner). This crate does not perform the S-N interpolation or produce FS_N/Nf itself.
- This crate calculates a critical-plane parameter, not FS_N, Nf, fatigue damage, crack growth, remaining life or a structural safety factor.
- Notch gradients, critical-distance corrections, residual stresses, stabilized cyclic plasticity and manufacturing effects are not yet implemented.
- Evidence digest syntax is checked; source bytes and FEA/measurement correctness are not authenticated.

## Local verification

```sh
cargo +1.96.0 fmt --manifest-path research/multiaxial-hcf-reference/Cargo.toml -- --check
cargo +1.96.0 test --manifest-path research/multiaxial-hcf-reference/Cargo.toml --locked
cargo +1.96.0 clippy --manifest-path research/multiaxial-hcf-reference/Cargo.toml --all-targets --locked -- -D warnings
```

Research tracking: Symtropy #4899. Synthetic reference tests qualify software behaviour only, not physical accuracy.
