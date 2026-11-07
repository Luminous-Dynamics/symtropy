# Interstellar Medium Exposure Reference v0.1

A dependency-free Rust analytical calculator for one declared spherical dust-grain size class traversing a spacecraft's projected cross-section over a constant-speed path.

It computes:
- grain mass from spherical radius and material density;
- Lorentz factor, relativistic kinetic energy and momentum per grain;
- Newtonian kinetic energy for comparison;
- expected encounters under a homogeneous independent Poisson model;
- probability of at least one encounter under that same assumption;
- expected intercepted mass and incident kinetic-energy bookkeeping.

It does **not** model how the grain's energy couples to a spacecraft, nor material erosion, crater formation, spallation, melting, shielding or mission survival.

## Run

```sh
cargo test
cargo run --release
cargo run --release -- --input fixtures/synthetic-4p24ly-10nm-dust-v0-1.input
```

## Equations

For spherical grains:
- mass: (m = (4/3)\pi r^3\rho)
- Lorentz factor: (\gamma = 1/\sqrt{1-\beta^2})
- relativistic kinetic energy: (E_k=(\gamma-1)mc^2)
- momentum: (p=\gamma m\beta c)

For homogeneous independent particles:
- expected impacts: (\lambda=nAL)
- probability of at least one impact: (P(N\geq1)=1-e^{-\lambda})

The Poisson model is an explicit assumption. It is not valid for spatially clustered or correlated dust without a different model. For several grain-size bins, evaluate each independently and combine only when their identity, units, source and spatial-model assumptions are compatible.

## Frozen synthetic fixture

The fixture uses:
- path length 4.24 light-years;
- spacecraft projected area 5.0 × 10^-11 m²;
- cruise speed 0.2c;
- spherical grain radius 10 nm;
- grain material density 3,000 kg/m³;
- grain number density 1.0 × 10^-6 m^-3.

It yields approximately:
- grain mass 1.25664 × 10^-20 kg;
- expected impacts 2.00567;
- probability of at least one impact under the Poisson assumption 0.86543;
- relativistic kinetic energy per grain 2.32892 × 10^-5 J.

These are synthetic controls chosen to exercise the equations and their sensitivity, not a source-pinned estimate for a real route or a prediction of Starshot survival.

## Research context

Hoang et al. (2017) modelled gas and dust damage for a 0.2c spacecraft under declared interstellar column and material assumptions (ApJ 837, 5; DOI 10.3847/1538-4357/aa5da6). Long (2023) published a Starshot gram-scale particle-bombardment analysis (JBIS 76, 262–272; DOI 10.59332/jbis-076-06-0262). A 2022 picogram-scale probe analysis highlights that encounter counts depend strongly on grain-size number density and projected cross-section.

These works do not share one universal particle distribution or material-response boundary. Reproduction must pin each study's exact inputs and output definition before numerical comparison.

## Evidence ceiling

Evidence class: synthetic analytical exposure bookkeeping. Full incident kinetic energy is not the same as energy deposited in the spacecraft material. No damage depth, erosion thickness, shielding mass, failure probability, survival probability, flight qualification or crewed mission conclusion is implied.
