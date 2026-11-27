# Sunshade Projected-Area Bound Reference v0.1

A dependency-free Rust reference for one analytical lemma under the finite-source geometric-optics programme.

## What it calculates

The inputs are the target-plane area `A_T` and the spatial integral of measured or independently calculated fractional attenuation over that finite domain:

`I_A = integral_T A(x) dA_target`

The evaluator rejects `I_A > A_T`, since local fractional attenuation cannot exceed one, and reports the mean attenuation `I_A / A_T`.

For source-to-shade distance `L_s`, shade-to-target distance `L_t`, and uniform scalar module transmittance `tau`, define:

`alpha = L_s / (L_s + L_t)`

The returned lower bound is:

`projected_mask_union_area >= alpha^2 * I_A / (1 - tau)`

The result is a lower bound on the **projected union area of the shade mask in one common shade plane**. It is not the physical membrane area for arbitrary tilted or non-coplanar modules; it is not the sum of physical module areas; it is not a mass estimate; and it is not a complete installed-system estimate.

## Derivation

Let `M` be the union of opaque-equivalent module footprints in the common shade plane. The source is a finite, uniformly bright disk. For each target-plane point `x`, the projected source disk at the shade plane has radius `b = L_t/(L_s + L_t) * R_sun` and centre `alpha * x`.

The geometric blocked fraction is:

`B(x) = (1 / (pi*b^2)) * integral_M 1(||y - alpha*x|| <= b) dA_y`

Integrating over the entire target plane and exchanging the nonnegative integrals gives:

`integral_R2 B(x) dA_x = |M| / alpha^2`

For a finite target domain `T`, the integral cannot exceed the entire-plane integral:

`integral_T B(x) dA_x <= |M| / alpha^2`

With common uniform transmission `tau`, local attenuation is `A(x) = (1 - tau) * B(x)`. Rearranging yields the bound implemented here.

The finite source radius cancels from the entire-plane integral identity. It still controls the spatial penumbra map and finite-target edge spill; the map must still be calculated or measured to supply `I_A`.

## Required assumptions

- Source, shade and target are parallel planes.
- A single static common shade plane and fixed positive distances are used.
- The source is a finite, uniformly bright circular disk.
- Straight-line geometric optics applies.
- All modules share the same scalar radiant transmittance.
- Scattering, diffraction, spectral variation, angle-dependent BRDF, module motion and multiple shade planes are outside the claim.

The factor `attenuation_effectiveness` in the earlier v0 equivalent-area equation is not automatically `1 - tau` or `alpha`. No mapping is assumed.

## Run

```sh
cargo fmt --manifest-path tools/sunshade-area-bound-reference/Cargo.toml -- --check
cargo test --manifest-path tools/sunshade-area-bound-reference/Cargo.toml
cargo clippy --manifest-path tools/sunshade-area-bound-reference/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/sunshade-area-bound-reference/Cargo.toml
```

The tests cover the closed-form bound, uniform-transmission scaling, the exact full-plane integral identity, distance-ratio scaling, zero attenuation, invalid inputs and overflow rejection.

A passing software test establishes implementation consistency with this narrow analytic contract only. It does not qualify the finite-source radiance map, real membrane optical properties, diffraction/scattering, orbital conditions or physical system mass.
