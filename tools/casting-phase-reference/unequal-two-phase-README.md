# Unequal-property two-phase Stefan analytical reference

Standalone Rust verification oracle for a one-dimensional two-phase solidification problem with distinct solid and liquid heat capacity, conductivity and thermal diffusivity.

It is a mathematical algorithm-verification reference, not a casting simulator or material database.

## Governing assumptions

- Equal density in solid and liquid; no volume change.
- Constant, phase-specific heat capacities and conductivities.
- Initially superheated liquid at T_inf.
- Fixed cold wall T_w at x=0.
- Sharp interface at melting temperature T_m.
- Conduction only: no advection, convection, flow, volumetric heat source or finite mold coupling.
- Semi-infinite analytical solution; a finite domain is used only to test remote-boundary adequacy.

Define alpha_s=k_s/(rho*c_s) and alpha_l=k_l/(rho*c_l). Interface continuity requires:

    s(t)=2*lambda_s*sqrt(alpha_s*t)=2*lambda_l*sqrt(alpha_l*t)

therefore lambda_l=lambda_s*sqrt(alpha_s/alpha_l).

The solid and liquid temperature fields are:

    T_s = T_w + (T_m-T_w) erf(x/(2 sqrt(alpha_s t))) / erf(lambda_s)
    T_l = T_inf + (T_m-T_inf) erfc(x/(2 sqrt(alpha_l t))) / erfc(lambda_l)

Set Ste_s=c_s(T_m-T_w)/L and Ste_l=c_l(T_inf-T_m)/L. The Stefan heat-flux jump condition reduces to:

    Ste_s * exp(-lambda_s^2) / (lambda_s * erf(lambda_s))
      - Ste_l * exp(-lambda_l^2) / (lambda_l * erfc(lambda_l))
      = sqrt(pi)

The scalar positive root is solved by bracketed bisection; no unconstrained Newton step can jump to a nonphysical branch.

## Frozen unequal-property fixture

rho=7800 kg/m³, c_s=600 J/(kg K), c_l=800 J/(kg K), k_s=25 W/(m K), k_l=18 W/(m K), L=270000 J/kg, T_w=-40 °C, T_m=0 °C, T_inf=100 °C, t=100 s, D=0.2 m.

High-precision independent values:
- alpha_s=5.34188034188034188034e-6 m²/s
- alpha_l=2.88461538461538461538e-6 m²/s
- lambda_s=0.1467466795409704487445
- lambda_l=0.1996969368461650935136
- interface s=0.00678336766139809364508 m
- wall heat=29,695,671.5556023427885 J/m²
- total enthalpy decrease agrees with wall heat to < 1e-54 J/m² in the high-precision reference calculation.

## Independent energy closure

The wall heat uses the time-integrated solid-side wall gradient:

    Q_wall = 2*k_s*(T_m-T_w)*sqrt(t)
             / (sqrt(pi)*sqrt(alpha_s)*erf(lambda_s))

The enthalpy decrease is implemented independently by integrating:
1. initial liquid enthalpy (latent + superheat) in the converted region;
2. final solid sensible enthalpy over that region;
3. remaining liquid superheat removal ahead of the interface.

The two expressions are compared every time the summary is produced. They do not share the same numerical integral implementation.

## Equal-property metamorphic limit

The second frozen case sets k_s=k_l and c_s=c_l to the published equal-property 2025 Stefan reference. The coupled lambdas must collapse to the same value 0.32239063292230086 and reproduce its 288-hour interface 0.5872254890905362 m.

## Build and run

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/two_phase_unequal.rs -o /tmp/two-phase-unequal-tests
    /tmp/two-phase-unequal-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/two_phase_unequal.rs -o /tmp/two-phase-unequal
    /tmp/two-phase-unequal --verify
    /tmp/two-phase-unequal --csv

## Claim ceiling

This is a 1D sharp-interface conduction reference with equal density and constant phase properties. It does not validate real alloy thermodynamics, temperature-dependent properties, a mushy zone, shrinkage, porosity, fluid flow, mold coupling, microstructure, defect prediction or production quality.
