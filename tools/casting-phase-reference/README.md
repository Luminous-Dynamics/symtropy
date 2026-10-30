# Casting phase-change analytical reference

This folder contains a dependency-free Rust analytical oracle for the one-phase planar Stefan freezing similarity problem. It is intentionally separate from the main Cargo/Bevy workspace and can be compiled directly with rustc.

## Scope

The reference assumes a semi-infinite domain that is initially liquid at the melting temperature, a wall stepped to a colder temperature at x = 0, a planar solid/liquid front, constant solid density/heat capacity/conductivity/latent heat, negligible liquid-side temperature gradient, no advection and no volumetric heat source.

It is not a casting process simulator, an alloy model, a numerical PDE solver, or physical validation.

For 0 < Ste <= 1:

- alpha = k / (rho * cp)
- Ste = cp * (T_m - T_wall) / L
- s(t) = 2 * lambda * sqrt(alpha * t)
- lambda * exp(lambda^2) * erf(lambda) = Ste / sqrt(pi)
- T(x,t) = T_wall + (T_m - T_wall) * erf(x/(2*sqrt(alpha*t))) / erf(lambda) in the solid, and T_m in the liquid.

The implementation uses a power series for erf only on |x| <= 1; the admitted initial profile ensures the similarity root and solid temperature coordinate remain in that interval. Unsupported values are rejected.

## Frozen synthetic known-answer case

| Quantity | Value |
|---|---:|
| Density | 1000 kg/m³ |
| Solid heat capacity | 2000 J/(kg·K) |
| Solid conductivity | 0.5 W/(m·K) |
| Latent heat | 200000 J/kg |
| Wall temperature | -20 °C |
| Melting temperature | 0 °C |
| Diffusivity | 2.5e-7 m²/s |
| Stefan number | 0.2 |
| Similarity root | 0.3064239053612112470964108171446327… |

Golden front positions are approximately 2.3735493646876174 mm at 60 s, 4.7470987293752349 mm at 240 s, and 9.4941974587504697 mm at 960 s. These are synthetic analytical reference values, not properties or outcomes of a real alloy.

## Build and run

From the repository root:

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/main.rs -o /tmp/casting-stefan-tests
    /tmp/casting-stefan-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/main.rs -o /tmp/casting-stefan-reference
    /tmp/casting-stefan-reference --verify
    /tmp/casting-stefan-reference --csv

The verification tests cover the golden root/front, boundary and interface temperatures, monotonicity, square-root-time similarity, invalid inputs, root residual and the bounded erf domain.

## Research references

- A semi-analytical benchmark for the Stefan problem in arbitrary dimension, International Journal of Numerical Methods for Heat & Fluid Flow (2022), DOI 10.1108/HFF-09-2021-0647.
- White, A Numerical Solution of the Enthalpy Formulation of the Stefan Problem, SIAM Journal on Numerical Analysis (1982), DOI 10.1137/0719083.

## Claim ceiling

A successful run establishes only that this reference implementation reproduces the frozen analytical known-answer case and its mathematical invariants. It does not qualify a candidate thermal solver, establish alloy properties, validate a casting process, or support a production/industry/SOTA claim.


## Energy closure check

For this one-phase similarity solution, the integrated heat removed through the cold wall must equal the sum of latent heat released at the advancing front and sensible cooling stored in the solid, per unit area:

`Q_wall(t) = rho * L * s(t) + rho * cp * integral_0^s [T_m - T(x,t)] dx`.

The executable evaluates both sides independently from the analytical expressions, emits them in the golden CSV, and checks their residual at 60, 240 and 960 seconds. This is an analytical conservation identity for the admitted idealized model; it is not physical validation of a foundry process.
