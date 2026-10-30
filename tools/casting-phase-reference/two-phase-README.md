# Two-phase superheated-liquid Stefan analytical reference

This standalone Rust oracle reconstructs the exact restricted 1D two-phase isothermal Stefan similarity case used by Peters et al. (2025), *A new implicit formulation of the enthalpy method using flag updates*, International Journal of Heat and Mass Transfer 249, 127166, DOI 10.1016/j.ijheatmasstransfer.2025.127166.

It is a mathematical reference, not a PDE solver, physical material database, or foundry validation result.

## Published 1D benchmark conditions

| Parameter | Value |
|---|---:|
| Density | 1 kg/m³ |
| Solid/liquid heat capacity | 2.5e6 J/(kg·K) |
| Solid/liquid conductivity | 2 W/(m·K) |
| Latent heat | 1e8 J/kg |
| Wall temperature | -10 °C |
| Melting temperature | 0 °C |
| Initial/far-field liquid temperature | 2 °C |
| Finite computation domain | D = 6 m |
| End time | 288 h |
| Boundary conditions | T(0,t)=-10 °C; dT/dx at D = 0 |
| Exact reference | Semi-infinite two-phase Stefan solution, D -> infinity |

The paper's text states N=48 gives dx=0.125 m, which implies 48 intervals across D=6 m; another part of its methods describes N equally spaced gridpoints with dx=D/(N-1). Treat this node/interval convention as an explicit reproduction ambiguity until the supplementary MATLAB code is inspected. Do not silently choose one convention when reproducing the paper's runtime/error tables.

## Exact solution

Because solid and liquid properties are equal, both phases share alpha = k/(rho*cp) = 8e-7 m²/s and one similarity parameter lambda.

Stefan numbers:

- Ste_solid = cp*(Tm-Tw)/L = 0.25
- Ste_liquid = cp*(T_infinity-Tm)/L = 0.05

Solve the positive root:

    lambda * exp(lambda^2) =
      (Ste_solid / erf(lambda) - Ste_liquid / erfc(lambda)) / sqrt(pi)

Then s(t) = 2*lambda*sqrt(alpha*t), with:

    T_s(x,t) = Tw + (Tm-Tw)*erf(x/(2*sqrt(alpha*t))) / erf(lambda)
    T_l(x,t) = T_infinity + (Tm-T_infinity)*erfc(x/(2*sqrt(alpha*t))) / erfc(lambda)

in the solid and liquid regions respectively.

## Frozen known answers

At 288 h:

| Observable | Reference |
|---|---:|
| Similarity parameter | 0.32239063292230086 |
| Interface position | 0.5872254890905362 m |
| T(0.25s) | -7.418693664209443 °C |
| T(0.50s) | -4.870670835179128 °C |
| T(1.25s) | 0.24583017128942997 °C |
| T(2s) | 0.8839591570978611 °C |
| Integrated cold-wall heat | 73,078,427.87840557 J/m² |

The independently evaluated initial-minus-final enthalpy change is 73,078,427.87840554 J/m², giving an absolute double-precision residual around 3e-8 J/m².

## Build and run

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/two_phase.rs -o /tmp/two-phase-tests
    /tmp/two-phase-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/two_phase.rs -o /tmp/two-phase
    /tmp/two-phase --verify
    /tmp/two-phase --csv > /tmp/two-phase.csv
    diff -u tools/casting-phase-reference/two-phase-expected.csv /tmp/two-phase.csv

## Why the benchmark requires multiple quality metrics

Peters et al. define the 1D interface by locating S = rho*L/2 and linearly interpolating between grid points. Their variable-time-step method fixes the interface at selected grid nodes, yielding zero interface error by construction while its temperature-field error is substantially larger. Therefore the Symtropy benchmark must score full temperature error, interface error, energy closure, stability and compute cost separately. An exact-looking interface alone is not a quality result.

The paper's table and error definitions are reproduced in the parent benchmark manifest. The official MATLAB supplementary files still need to be fetched, pinned and inspected before calling any port a reproduction of the authors' implementation.

## Claim ceiling

The temperature evaluator uses the convergent erf/erfc power series only for similarity coordinates in [0, 1]; requests outside that declared range fail explicitly. The frozen profile samples stay inside this admitted domain.\n\nThis oracle covers only the published equal-property two-phase planar similarity problem. It does not represent alloy phase intervals, temperature-dependent properties, fluid convection, mold contact, multidimensional casting, microstructure, porosity, shrinkage, or physical production qualification.
