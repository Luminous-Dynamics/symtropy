# Space Collider Reference Checks v0.1

A dependency-free analytical reference calculator for four narrow checks raised by the May 2026 space-based-collider preprint:

1. Symmetric head-on collider energy convention: centre-of-mass energy is twice the energy per beam.
2. Lorentz factor from explicitly typed per-beam energy.
3. Magnetic-rigidity radius estimate using the preprint's stated filling-factor relation.
4. Orbital revolution frequency for a declared ring circumference.

## Run

```sh
cargo test
cargo run --release
```

## Numerical finding

The preprint defines (E_{CM}=2E_{beam}) for symmetric head-on proton beams and gives the illustrative LHC design reference as 14 TeV centre-of-mass (7 TeV per beam). It then states a 1 PeV example and claims a Lorentz-factor ratio of about 140.

Under the stated centre-of-mass convention:
- 1 PeV centre-of-mass means 500 TeV per beam.
- 14 TeV LHC centre-of-mass means 7 TeV per beam.
- Their Lorentz-factor ratio is therefore 500/7 = 71.428571.

A ratio of 142.857143 instead results if 1 PeV is interpreted as **per-beam** energy. That would be a 2 PeV centre-of-mass collider. The preprint needs to clarify which convention is intended before using the gamma factor in its luminosity-compensation argument.

This audit does not establish that the collider is feasible or that the paper's broader concept is invalid. It isolates an energy-convention ambiguity in one numeric compensation term.

## Other reference checks

- 47,000 km circumference: (f=c/C approx 6.37856) Hz, consistent with the paper's rounded ~6.5 Hz.
- 1 PeV centre-of-mass, 2 T dipoles, 0.7 dipole filling factor: required radius ≈1,191.30 km, matching the paper's table's ≈1,190 km.
- 47,000 km circumference at 2 T and 0.7 fill factor: rigidity-only maximum centre-of-mass energy ≈6.279 PeV under the stated simplified formula.

These are analytic consistency checks on the paper's equations and declared conventions. They are not a FODO lattice calculation, beam-dynamics simulation, luminosity result, detector model, cryogenic design, orbital dynamics result, or engineering feasibility finding.

## Evidence boundary

- Evidence class: analytical reference checks.
- No external dependencies.
- Input values are hardcoded in the small reference executable and unit tests; they are not a source-data corpus.
- This does not establish beam stability, luminosity, beam–beam tune shift, formation tolerance, passive cooling, power feasibility, launch mass, cost, safety or orbital-ring feasibility.
