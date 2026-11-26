# Sunshade Finite-Source Reference v0.1

A standalone, dependency-free Rust analytical kernel for the geometric shadow cast by **one circular module** onto **one target-plane point** by a finite, uniformly bright circular Sun.

This is the first executable slice under [SUNSHADE-FINITE-SOURCE-TRANSFER-001 #7662](https://github.com/Luminous-Dynamics/symtropy/issues/7662). It is intended to correct the missing geometry boundary in the earlier equivalent target-plane area proxy. It is not a full planetary-sunshade model.

## Geometry

The model uses three parallel planes:

- Source plane at (z=-L_s), with a circular solar disk of radius (R_s) centred on the z-axis.
- Physical shade/module plane at (z=0).
- Target plane at (z=+L_t), with a selected target point ((x_t,y_t)).

Straight rays project the finite solar disk onto the shade plane with:

```text
r_projected = L_t / (L_s + L_t) * R_s
c_projected = L_s / (L_s + L_t) * (x_t, y_t)
```

The local geometric blocked fraction is the area of intersection between that projected disk and the physical circular module, divided by the projected solar-disk area.

For module radiant transmittance `tau`:

```text
attenuation_fraction = geometric_blocked_fraction * (1 - tau)
target_transmittance = 1 - attenuation_fraction
```

The circle-intersection area is computed analytically, including no-overlap and complete-containment cases.

## Assumptions and claim ceiling

This v0 assumes:

- uniform solar-disk radiance (no limb darkening);
- parallel source, shade and target planes;
- a circular module parallel to those planes;
- one module only;
- geometric optics and straight ray propagation;
- a local target point, not integrated Earth-disk service;
- scalar module transmittance, not spectral BRDF/reflectance/scattering.

It does **not** model diffraction, wave optics, non-circular shapes, multi-module overlap unions, module attitude, orbital motion, target surface curvature, solar spectrum, limb darkening, climate effects, or complete installed system mass.

A passing test proves only the stated geometric calculation. It does not validate a physical shade material, L1 geometry, orbital stability, planetary-scale radiative service, climate response or deployment feasibility.

## Run

```sh
cargo fmt --manifest-path tools/sunshade-finite-source-reference/Cargo.toml -- --check
cargo test --manifest-path tools/sunshade-finite-source-reference/Cargo.toml
cargo clippy --manifest-path tools/sunshade-finite-source-reference/Cargo.toml --all-targets -- -D warnings
```

## Tests

The frozen tests cover no overlap, complete blockage, a module contained within the projected solar disk, analytic partial overlap of equal circles, external/internal tangency, partial transmittance, finite-distance projection, and invalid/non-finite inputs.

The next increments are multiple-module union geometry, target-disk spatial integration, limb-darkened spectral radiance, module attitude, optical material response and independent bench shadow-map validation. Diffraction must be treated as a separate optical regime rather than silently absorbed into a scalar effectiveness factor.
