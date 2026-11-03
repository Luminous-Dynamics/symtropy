# Soil EM propagation reference

A dependency-free Rust reference for plane-wave propagation in a homogeneous,
isotropic, linear lossy medium under conventional Maxwell electrodynamics.

## Scope

This package calculates the exact homogeneous-medium plane-wave attenuation
and phase constants from frequency, relative permittivity, relative
permeability and conductivity. It reports amplitude attenuation, field and
power ratios, wavelength, conduction/displacement current ratio and skin
depth. It also exposes the familiar good-conductor approximation, but only
when the conduction ratio is at least 100.

It also contains an ideal hemispherical-electrode spreading-impedance
reference for a hemispherical equipotential electrode flush with an insulating
plane over a uniform conductive half-space. Its complex impedance is
`1 / (2*pi*a*(sigma + j*omega*epsilon))`; the DC limit is `1/(2*pi*a*sigma)`.
This ideal one-electrode result is not a two-terminal ground resistance.

This is a **material propagation reference**, not a model of a soil electrode,
ground rod, lossy half-space with an earth return electrode, layered geology,
near-field coupling, surface-wave propagation, Earth-ionosphere modes or the
Wardenclyffe installation. Those require distinct geometry and boundary-value
models. Do not reverse-infer soil conductivity from Tesla's diary-reported
14-ohm two-electrode measurement.

## Run

From this directory:

    cargo test
    cargo run --release --bin material_sweep > soil-propagation.csv

The sweep writes stdout CSV over a declared conductivity/frequency grid.
Parameters are illustrative sensitivity ranges, not measurements from
Colorado Springs or Wardenclyffe.

## Analytical quantities

For angular frequency omega, permittivity epsilon, permeability mu and
conductivity sigma, let r = sigma/(omega*epsilon). The exact homogeneous
plane-wave expressions are:

    alpha = omega * sqrt(mu*epsilon/2) * r / sqrt(sqrt(1+r*r) + 1)
    beta  = omega * sqrt(mu*epsilon/2) * sqrt(sqrt(1+r*r) + 1)

Alpha is amplitude attenuation in Np/m; beta is phase constant in rad/m.
The field amplitude decays as exp(-alpha*d); power decays as exp(-2*alpha*d).
The good-conductor skin-depth approximation is sqrt(2/(omega*mu*sigma)),
but it is exposed only for sigma/(omega*epsilon) >= 100.

## Known-answer example

For relative permittivity 10, relative permeability 1, conductivity 0.01 S/m
and frequency 10 kHz, the model gives a skin depth of approximately 50.3432 m
and an amplitude attenuation of approximately 0.17253 dB/m.

These are consequences of the chosen homogeneous material parameters; they
are not actual soil readings, electrode impedance predictions or estimates
of the Tesla apparatus.

## Engineering use

Use this crate as an analytical check on material propagation terms in a
later frequency-domain field solver. Ground-return/electrode work should
separately include electrode geometry, contact impedance, soil layering,
spatial current spreading, boundary conditions and measured calibration.
Compare solver and analytic results only on a matched physical problem.

## Validation status

Nine Rust unit tests are authored across plane-wave and ideal electrode references. They have not been compiled or executed in
this authoring environment because Cargo/rustc are unavailable. No CI or
numerical qualification pass is claimed.