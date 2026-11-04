# Tesla US787,412 numeric claim audit

A dependency-free Rust arithmetic audit of numerical claims stated in Nikola
Tesla's US787,412 patent, filed in 1900 and granted in 1905.

Primary source: https://patents.google.com/patent/US787412A/en

## Scope

This package reproduces dimensional arithmetic from patent statements:

- stated mean speed 471,240 km/s;
- estimated return interval 0.08484 s;
- stated wavelength range 25–70 km;
- comparison with the exact SI speed of light and mean Earth radius.

It does **not** reproduce a physical measurement, identify the wave mode, solve
Maxwell's equations, establish a phase/group/front/energy velocity, or support
superluminal information transfer. Patent statements are preserved as claims
or estimates, not promoted to observed facts.

## Derived reference values

- Stated speed: 471,240,000 m/s.
- Ratio to c: approximately 1.5718874422.
- Speed multiplied by the estimated interval: 39,980,001.6 m.
- Mean Earth circumference (radius 6,371 km): approximately 40,030,173.6 m.
- Implied path/circumference ratio: approximately 0.9987466457.
- Frequencies implied by 25–70 km at the patent speed: 18.8496 kHz down to 6.732 kHz.
- Frequencies for the same wavelengths at c: 11.9917 kHz down to 4.2827 kHz.

The arithmetic is internally compatible with a round-trip distance near one
Earth circumference and wavelengths implying frequencies below 20 kHz. That
does not validate the underlying wavelength, propagation mechanism, travel
time or physical velocity. The patent calls the interval/velocity an estimate.

The Earth-diameter / odd-quarter-wavelength relation corresponds to a very
large integer-like quarter-wave factor range of about 728 to 2,039 over the
patent's wavelength interval. This output is a continuous arithmetic range;
the program does not select or assert an odd integer resonance.

## Run

    cargo test
    cargo run --release --bin audit > patent-arithmetic.csv

The output is a CSV of values tagged by classification (`patent_assertion`,
`patent_estimate`, `derived_arithmetic` or `reference_constant`).

## Validation status

Four deterministic Rust unit tests are authored. They have not been compiled
or executed in this authoring environment because Cargo/rustc are unavailable.
No CI or physical validation pass is claimed.