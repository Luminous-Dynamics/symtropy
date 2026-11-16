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


## Beam, luminosity and operating-budget closure

The reference executable now includes a second, separate arithmetic audit for the 2022 lunar Circular Collider (CCM) parameter set. It closes the paper's source inputs across:

- revolution frequency and actual colliding-bunch-pair rate;
- stored bunch population and reconstructed beam current;
- the paper's round-beam, head-on tune-shift luminosity relation;
- interactions per crossing for an explicitly named total cross section;
- burn-off beam lifetime;
- synchrotron-radiation energy loss per turn from per-beam power/current;
- scheduled-time and availability conversion to annual integrated luminosity.

It also reports a pinned FCC-hh CDR baseline row (97.75 km, 100 TeV centre-of-mass, 16 T, 10,600 bunches, 0.5 A, 2.4 MW/beam) and its derived revolution/collision rates. FCC baseline and ultimate luminosity/event-crossing figures remain separate source values.

### First arithmetic findings

Using the lunar paper's rounded inputs:

- 11,000 km circumference gives 27.2539 revolutions/s.
- 1.2 million bunches × 10^11 protons gives 0.524 A from the bunch population, consistent with the rounded 0.5 A.
- 1.2 million colliding pairs per IP gives 32.705 MHz crossings/IP.
- 25 ns slots would permit 40 MHz only for a completely filled ring; the source bunch count corresponds to an approximately 81.8% nominal fill fraction.
- The tune-shift formula produces 1.82 × 10^38 cm^-2 s^-1 using the rounded 0.5 A input, or 1.91 × 10^38 when using the bunch-derived current, close to the paper's approximately 2 × 10^38.
- 283 mb total cross section with the reported luminosity implies about 1.73 million total-cross-section interactions per crossing. This is a rate proxy, not a detector-accepted pile-up count.
- The paper's rounded approximately 1 million interactions/crossing implies an effective cross section near 164 mb at the same collision rate. The tool records this as an implied value, not as a source-certified inelastic cross section. The event definition/cross section behind the rounded table figure needs clarification.
- The burn-off equation gives approximately 17.67 minutes, consistent with the paper's 18-minute estimate.
- 8.5 TW per beam / 0.5 A gives approximately 17 TeV lost per proton per turn.
- 160 scheduled physics days × 70% availability gives approximately 1,935 ab^-1 per year at the stated peak luminosity and effective operating fraction 1.0.

The CERN-2025-006 FCC-hh baseline gives about 3,066.93 turns/s and about 32.51 million collision crossings/s for 10,600 colliding bunch pairs/IP. The lunar and FCC baselines therefore have similar nominal per-IP crossing rates in these source examples, despite their very different ring circumferences and bunch counts. The lunar source's much larger luminosity is consequently expressed as an enormous interactions-per-crossing burden, not merely as a higher collision frequency.

These are arithmetic closures and source-consistency diagnostics. They do not validate the lunar paper's assumed proton-beam tune-shift limit, FODO/final-focus optics, detector response, top-up injection, RF delivery, magnet performance or the actual feasibility of sustaining 8.5 TW per beam.

Sources:
- Beacham & Zimmermann, 2022 lunar collider paper: https://doi.org/10.1088/1367-2630/ac4921
- CERN FCC-hh Extended Conceptual Design Report, CERN-2025-006: https://doi.org/10.23731/CYRM-2025-006
- CERN luminosity calculator: https://lpc.web.cern.ch/lumiCalc.html
- PDG 2025 accelerator physics and collider review: https://pdg.lbl.gov/2025/reviews/rpp2025-rev-accel-phys-colliders.pdf

The Rust evaluator intentionally keeps total-cross-section interactions, inelastic process pile-up and detector-accepted events distinct. No unprovided inelastic cross section or detector acceptance is silently supplied.


### Stored beam energy: a first-order machine-protection constraint

The same Rust evaluator now calculates stored energy by two independent routes:
- bunch population × particles per bunch × per-beam particle energy;
- reported beam current × per-beam particle energy / revolution frequency.

For the lunar CCM source values, the bunch route gives approximately:
- **134.583 TJ per beam**;
- **269.166 TJ for both counter-rotating beams**.

The rounded 0.5 A current route gives a value about 4.6% lower, consistent with the source's rounded current. The evaluator reports this residual rather than silently forcing the current to match the bunch-derived value.

Against the FCC-hh CDR reference of about 8.4 GJ per beam, the CCM first-order value is about **16,022 times larger**. This comparison is a machine-protection red flag, not a claim that the two designs have equivalent dump arrangements. It must lead to a credible beam abort, extraction, dilution, dump core, window, shielding, cooling, repeated-abort and common-cause fault analysis. No generic absorber or uniform energy-density estimate can be treated as a qualified dump design.

The current tool reports the arithmetic only; actual particle-shower energy deposition, thermal shock, activation, radiation shielding and mechanical survivability must come from pinned specialist tools and exact dump geometry/material/source terms (for example FLUKA or Geant4 plus qualified structural/thermal analysis).

References:
- Lunar CCM source beam parameters: https://arxiv.org/html/2106.02048v3
- FCC-hh 2025 CDR, stored energy per beam: https://doi.org/10.23731/CYRM-2025-006
- Follow-on research owner: [SPACE-COLLIDER-STORED-ENERGY-DUMP-001 #5572](https://github.com/Luminous-Dynamics/symtropy/issues/5572)


## Reproducibility receipt

The focused GitHub Actions workflow executes the report twice and requires byte-identical JSON output. It parses the complete JSON document (including the nested luminosity and stored-energy closure), checks the expected numerical ranges, and retains:
- the final JSON report;
- an evidence manifest binding the exact checked-out commit, pinned Rust/Cargo versions, source-file SHA-256 fingerprints and report SHA-256.

The two files are uploaded as a run artifact named `space-collider-reference-<exact-commit>`. The report is source-arithmetic evidence only; the manifest makes its software/input lineage reproducible and does not turn it into physical validation.
