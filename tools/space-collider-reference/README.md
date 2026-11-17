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
- The paper reports only approximately 1 million interactions/crossing. The independently recomputed 1.73 million total-cross-section proxy is consistent at that order of magnitude. No separate inelastic cross section or detector-accepted event count is inferred from that rounded table entry.
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


## Muon species and ring-circumference scaling screen

The existing executable now includes a separate analytical species-scaling object for the 2026 NLASG 10 TeV muon-collider reference and two Moon-scale extrapolations. It uses the same dependency-free Rust crate, not a second accelerator calculator.

### Source reference and baseline reconstruction

The 5 November 2026 NLASG report (FERMILAB-PUB-26-0741-TD / arXiv:2611.06680, Table 1) gives 10 TeV centre-of-mass, 11.4 km circumference, 14 T peak arc field, 1.8e12 muons/bunch, 5 Hz repetition, 14.4 MW beam power and 1,363-turn luminosity lifetime. The report also stresses that end-to-end muon production, cooling, acceleration and collisions have not been demonstrated as an integrated system.

The screen infers an **effective isomagnetic fill fraction ~0.656593** from 5 TeV/beam, 14 T and 11.4 km. This is a geometric proxy using the peak field as a uniform effective arc field, not a real lattice fill map. Under that declared approximation it obtains:
- muon Lorentz factor ~47,322;
- mean lab lifetime ~0.103966 s;
- particle decay e-fold ~2,734 turns;
- two-beam luminosity decay e-fold ~1,367 turns, close to the source's 1,363-turn value;
- 7.210 MW injection beam power per beam / 14.420 MW both beams, consistent with the rounded 14.4 MW table value;
- classical isomagnetic synchrotron loss ~25.40 MeV/turn.

The 2025 MuCol consolidated-parameter report gives a 27.2 MeV/turn design target for its 10 TeV case. The ~6.6% difference is retained as a source/model comparison: the screening approximation uses the NLASG table's 14 T peak field and a uniform bending-radius proxy, whereas the real radiation loss depends on the lattice radiation integral I2. This is not silently calibrated away.

### Two different lunar extrapolations

**Fixed beam energy (5 TeV/beam):** keep the bending radius/field and energy unchanged while stretching circumference to 11,000 km. The implied dipole fill fraction falls to about 0.00068 and the muon survives only ~2.83 turns per decay e-fold; the luminosity e-fold is ~1.42 turns. This is a poor use of a very large ring unless the energy is increased.

**Fixed field and source fill fraction:** keep 14 T and ~0.6566 dipole fill. At 11,000 km, the rigidity screen implies ~4.8246 PeV/beam (~9.649 PeV centre-of-mass). Both lab lifetime and revolution period scale linearly with circumference, so the number of turns per decay e-fold remains ~2,734. But the classical isomagnetic synchrotron loss scales as C^3 and reaches:
- ~22.82 PeV lost per muon per turn;
- ~4.730 beam energies per turn;
- a formal constant-energy RF replacement ratio ~12,932 times the nominal per-beam injection power;
- a formal synchrotron power scale ~89.97 TW per beam under the continuous-repetition exponential-decay model.

That scenario has **no fixed-energy circular equilibrium under the declared model**: it would need more than one beam energy replenished each turn. The formal power is a diagnostic extrapolation, not a sustainable machine operating point.

### A useful non-obvious invariant—and its limit

At fixed field, fill fraction, muons/bunch and repetition rate, beam energy grows with circumference while muon lifetime grows in the same proportion. The number of decay turns is therefore invariant. The total energy carried into muon decay products grows with circumference, but its simple ring-averaged energy-flow-per-metre remains approximately constant because both total decay energy flow and circumference scale linearly.

The report therefore outputs decay rate and decay-product energy flow separately from deposited heat. It does **not** infer that this energy is deposited in cold masses: the partition among electrons, photons, hadrons, neutrinos, shielding and cold mass requires transport simulation. The NLASG report's ~500 W/m cold-mass beam-induced deposition is a separate source quantity, not directly equated with this energy-flow diagnostic.

### Synchrotron thresholds and species comparison

Under the same isomagnetic fixed-field scaling, fractional synchrotron energy loss per turn grows as C^2. With the NLASG-derived baseline geometry, the screen finds:
- ~1% loss/turn at ~506 km;
- ~5% at ~1,132 km;
- ~10% at ~1,600 km;
- ~100% at ~5,058 km.

At the Moon-scale fixed-field energy, muon synchrotron loss is about 6,200 times the proton loss at the same energy/radius because the scaling is proportional to mass^-4. Electron radiation is far larger; the electron quantum parameter chi exceeds one in this Moon-scale comparator, so the classical electron loss is deliberately labelled invalid without a quantum synchrotron model. The muon chi remains ~3.39e-6, which supports the classical photon-emission approximation for this first-order radiation screen; it does not validate lattice or beam equilibrium.

### Claim ceiling

This is a kinematic lifetime / rigidity / classical-radiation screen using a uniform effective bending-radius model. It does not model 6D cooling, production and acceleration-chain survival, beam-beam dynamics, RF cavity layout, non-isomagnetic lattice I2, magnet shielding/decay deposition, detector backgrounds, neutrino dose or civil/site feasibility. For detailed work, use the exact MuCol/FCC design sources, Bmad/MAD-X/Xsuite/SixTrack beam and lattice models, and FLUKA/Geant4 transport with pinned geometries and physics lists.

Sources:
- NLASG 10 TeV muon collider report, 5 November 2026: https://lss.fnal.gov/archive/2026/pub/fermilab-pub-26-0741-td.pdf
- MuCol Consolidated Parameters, 2025, including 27.2 MeV/turn target: https://doi.org/10.5281/zenodo.17476875
- Synchrotron radiation relation / mass and energy scaling: https://pdg.lbl.gov/2009/download/rpp-2008-plB667.pdf
- Bmad radiation integral and stochastic emission reference: https://www.classe.cornell.edu/bmad/bmad-manual-2026-03-09.pdf
- Follow-on qualification owner: [SPACE-COLLIDER-MUON-SCALING-001 #5583](https://github.com/Luminous-Dynamics/symtropy/issues/5583)


### Integrated muon survival through the four reference RCS stages

The same Rust executable now integrates the muon decay hazard through the four-stage 63 GeV → 5 TeV rapid-cycling synchrotron chain described by Soubirou & Chancé (arXiv:2611.02865, 2 November 2026) and the MuCol 2025 consolidated parameter report.

For a linear total-energy ramp \(E(t)=E_{in}+(E_{out}-E_{in})t/T\), the exact decay exponent is:

\[
\Lambda = \frac{m_\mu}{\tau_0}\frac{T}{E_{out}-E_{in}}
\ln\left(\frac{E_{out}}{E_{in}}\right),
\qquad S=e^{-\Lambda}.
\]

The four source stages and calculated survival are:

| Stage | Energy (GeV) | Time (ms) | Turns | Calculated survival |
|---|---:|---:|---:|---:|
| RCS1 | 63 → 314 | 0.34 | 17 | 90.0649% |
| RCS2 | 314 → 750 | 1.10 | 55 | 89.9745% |
| RCS3 | 750 → 1,500 | 2.37 | 66 | 90.0019% |
| RCS4 | 1,500 → 5,000 | 6.37 | 55 | 89.9981% |

The complete chain takes 10.18 ms and 193 turns. Multiplying the four integrated stage results gives **65.6387% survival**. This is close to the source table's *assumed* 90% per ring / 65.61% cumulative survival. The code does not force any stage to 90%; the small differences arise from integrating the stated continuous energy ramp and rounded stage durations.

The analytic logarithmic integral is independently tested against a 20,000-interval composite-Simpson numerical integral. Focused CI asserts the four-stage count, total time, total turns and survival interval. This closes only the decay arithmetic for the source ramp profile; it does not validate transverse/longitudinal acceptance, magnet tracking, RF, emittance growth or stage transmission.

The 2026 rapid-acceleration study shows why the latter are still critical: RCS1's 8-arc RF arrangement exhibits over 500% horizontal emittance growth in its nominal case, whereas the 32-arc lattice reduces it to ~13%, and its energy–magnet-field mismatch study reports ~6.8% horizontal emittance growth for RCS1 under 32-arc conditions before further correction. That is a beam-quality issue, not a muon-decay survival discrepancy.

Sources:
- Soubirou & Chancé, arXiv:2611.02865, Table 1 and RCS1 tracking results: https://arxiv.org/abs/2611.02865
- MuCol Consolidated Parameters, 2025: https://doi.org/10.5281/zenodo.17476875


## Muon plasma-wakefield source-gap screen

The JSON report now includes a machine-readable screen for the published 2026 electron-beam-driven muon plasma-wakefield result (Wang et al., *Physical Review Accelerators and Beams* 29, 021302, DOI 10.1103/wq5g-mtjq). This is **not a WarpX reproduction**. It records the published headline values and their limitations for downstream research selection.

The paper-reported setup uses a 23 GeV electron driver and a 300 MeV muon injection energy, with a headline output at 40 GeV. The paper reports 18.85% final energy spread and 8.86 mm mrad normalized emittance.

The screen compares those headline values to the MuCol 10 TeV reference values of 0.1% rms high-energy injection energy spread and 25 μm normalized transverse rms emittance:
- energy-spread ratio: **188.5×**, if the definitions and statistical conventions are comparable;
- 8.86 mm mrad converts to 8,860 μm mrad; conditional emittance ratio: **354.4×**, if the plane, rms/normalization and averaging definitions are comparable.

These ratios are screening gaps, not harmonized machine-performance comparisons. The more consequential caveat is that the paper explicitly calculates the headline 300 MeV→40 GeV acceleration with **zero initial energy spread and zero initial emittance**. A separate finite-emittance injection study reports that large-divergence particles can be lost when their oscillation amplitude exceeds the plasma-bubble radius. The finite-source capture efficiency and the headline acceleration output therefore must not be silently conflated.

The paper's 3D WarpX PIC setup reports a 23 GeV electron driver, a plasma density ramp from 1e23 to 7e23 m^-3, a 0.1 mm up-ramp, and a 256×256×512 grid in an 800×800×750 μm moving window. Its supporting simulation data were **not publicly available at publication** and were stated to be available from the authors on reasonable request. Until the input decks and raw/source distributions are admitted, any independent recreation is an independent reconstruction, not exact reproduction.

The next benchmark is finite-source end-to-end performance: capture fraction, full 6D phase space, tails/losses, surviving muons, and downstream RF/RLA/RCS/FFA acceptance. High gradient alone is not a collider-acceleration result.

References:
- Wang et al., *Muon injection and acceleration via ultraintense electron beam-driven plasma wakefield*, PRAB 29, 021302 (2026): https://doi.org/10.1103/wq5g-mtjq
- MuCol Consolidated Parameters, 2025: https://doi.org/10.5281/zenodo.17476875
- Follow-on qualification owner: [SPACE-COLLIDER-MUON-PLASMA-ACCEL-001 #5641](https://github.com/Luminous-Dynamics/symtropy/issues/5641)


## Four-stage RCS horizontal-emittance budget screen

The report now includes a separate source-table arithmetic object for the Soubirou–Chancé 2026 RCS energy–magnetic-field mismatch study (arXiv:2611.02865, Table III).

Its 32-arc case reports horizontal emittance growth of 6.8%, 0.4%, 1.0% and 1.5% across RCS1–RCS4. If those stage-local percentages are applied sequentially to the same propagated emittance definition, the conditional composition is:

- additive first-order sum: 9.700000%;
- multiplicative composition: **9.92396408%**;
- share of the 10% nominal budget consumed: **99.2396408%**;
- remaining arithmetic headroom: **0.07603592 percentage points**.

This is deliberately marked "conditional_sequential_composition" and "physical_qualification_status: not_evaluated". It is not a full start-to-end tracking result: inter-stage matching, corrections, distribution changes, losses and correlations may invalidate direct compounding.

The report keeps the separate source cases distinct:
- the 8-arc RF-distribution comparison (>500% horizontal emittance growth);
- the 32-arc RF-distribution comparison (~13% under the stated linear-matching setup);
- the Table III 32-arc energy–magnetic-field mismatch values used for the conditional arithmetic.

Negative vertical/longitudinal changes are preserved as source observations and are not treated as evidence of true beam cooling. Ramp-jitter observations are carried separately; negligible extra emittance growth at 100 ppm does not mean zero orbit or optics perturbation.

The source-bound component qualifies transcription and arithmetic only. It does not replace RCS lattice tracking or prove that the full chain meets the physical emittance budget.


## Local engineered-ring host comparison

The JSON report now contains \`local_ring_screen\`, a 12-case first-order comparison for 50 km, 100 km and 150 km circumferences at 8.33 T, 14 T, 16 T and 20 T, with a declared 0.67 dipole fill fraction.

**Key geometry result:** at 97.75 km, 16 T and 0.67 fill, the simplified rigidity relation gives 99.9959 TeV centre-of-mass energy. At 100 km, it gives 102.2976 TeV CM. This demonstrates why local machine circumference should be optimized independently of the host body's radius.

**FCC-hh scale check:** at the FCC-hh conceptual geometry (97.75 km / 16 T), the simplified 0.67-fill, 0.5 A screen gives:
- 99.9959 TeV CM versus the source's 100 TeV;
- 2.3339 MW proton synchrotron power per beam versus the source's approximately 2.4 MW.

The differences are retained unadjusted (about -0.004% in the rigidity energy and -2.76% in SR power). Actual accelerator lattices require the radiation integral and detailed beam parameters; this is not a replacement for the FCC lattice model.

**Muon scaling is distinct.** At 100 km / 16 T / 0.67, the screen gives about 102.3 TeV CM, about 3,188 muon decay e-fold turns and about 1,594 two-beam luminosity decay e-fold turns in the simplified independent-decay model. With the NLASG 1.8e12-muon bunch and 5 Hz repetition profile, the illustrative per-beam injection power is about 73.75 MW and the formal isomagnetic SR power about 142.9 MW. This ratio is an analytical screen using a continuous repetition/decay approximation, not a machine load prediction or a credible collider operating point.

The 20 T entries are explicitly labelled HTS stretch cases, not production-qualified magnets. A 14 T / 16 T field label similarly does not imply aperture, field quality, protection, ramp-rate, manufacturing-yield or cost qualification.

The calculation does not model lattice optics, beam-beam dynamics, usable luminosity, emittance evolution, RF systems, detector acceptance, beam dump, radiation transport, neutrino dose, terrain/site, utilities or radiator sizing. These are retained under their existing specialist owners, including [#5529](https://github.com/Luminous-Dynamics/symtropy/issues/5529), [#5583](https://github.com/Luminous-Dynamics/symtropy/issues/5583), [#5907](https://github.com/Luminous-Dynamics/symtropy/issues/5907) and the new [#5924 local-ring comparison](https://github.com/Luminous-Dynamics/symtropy/issues/5924).


### Dipole-fill semantics and sensitivity

The model's \`0.67\` is a **ring-wide effective bending fraction**: total dipole bending length divided by full machine circumference. It is not the fraction of an arc cell occupied by dipoles.

For the 2019 FCC-hh CDR geometry, the cited machine has approximately 82 km of arcs in a 97.75 km ring and a dipole packing fraction of 0.80 within the arcs. The equivalent ring-wide fraction is:

\[
f_{ring}=0.80\times\frac{82.0}{97.75}=0.6710997.
\]

The common 0.67 matrix therefore approximates the FCC source geometry, but does not silently equate its arc-cell fill factor with the whole-ring bending fraction. The report exposes the conversion explicitly.

A 97.75 km / 16 T fill sensitivity is also included at ring-wide fractions 0.60, 0.67, 0.6710997 (FCC-derived) and 0.75:
- 0.60: ~89.55 TeV CM and ~1.68 MW/beam at 0.5 A;
- 0.67: ~100.00 TeV CM and ~2.33 MW/beam;
- 0.6710997: ~100.16 TeV CM and ~2.35 MW/beam;
- 0.75: ~111.94 TeV CM and ~3.27 MW/beam.

These values are first-order uniform-bend comparisons. The source FCC 2.4 MW per beam is based on the actual lattice; our simplified radiation relation does not reproduce the detailed radiation integral or lattice configuration. The difference remains a model-discrepancy diagnostic, not a calibration target.

Source: [FCC-hh Extended Conceptual Design Report, CERN-2025-006](https://doi.org/10.23731/CYRM-2025-006).


### FCC-hh reference vintage: CDR versus 2025 updated study

The 97.75 km / 16 T / 100 TeV / approximately 2.4 MW-per-beam row is the **FCC-hh CDR configuration** and is retained as an ambitious 100 TeV comparator. It should not be described as the latest revised study reference.

The 2025 updated FCC-hh study also presents a lower-field / lower-energy reference at approximately **90.7 km / 14 T / 85 TeV CM**. Its table gives a 76.9 km arc length and 0.83 dipole packing within those arcs. Converting those values to the full-ring effective bending fraction gives:

\[
f_{ring,2025}=0.83\\times\\frac{76.9}{90.7}\\approx0.7037.
\]

This distinction matters:
- 100 TeV / 16 T is the CDR stretch comparator used by the local-ring executable.
- 85 TeV / 14 T is the updated 2025 study reference and a useful more conservative magnet-field comparator.
- The 14.1 T demonstrated dipole and the continuing 16 T Nb3Sn / 20 T HTS development programme should not be conflated with a production-qualified 20 T collider magnet.

The comparison therefore needs to report both reference vintages rather than describing the 100 TeV CDR as the sole current industry baseline.

Sources:
- [CERN FCC-hh CDR, CERN-2025-006](https://doi.org/10.23731/CYRM-2025-006)
- [2025 FCC-hh parameter update](https://indico.cern.ch/event/1520755/attachments/3023021/5342322/FCC_FinalReport_Vol2-HFM.pdf)
- [CERN High Field Magnets programme](https://hfm.web.cern.ch/)


The executable report now includes the 2025 updated FCC-hh comparator directly:
- source geometry: 90.7 km ring, 76.9 km arcs, 0.83 arc-cell dipole packing, 14 T and 85 TeV CM;
- converted ring-wide effective fill: 0.7037155;
- screen: ~85.2714 TeV CM, about +0.32% relative to the 85 TeV source value;
- the uniform-bend 0.5 A proton screen emits ~1.266 MW/beam, while the source table labels its synchrotron entry as MW **per ring**. The report explicitly marks these unlike conventions as not directly compared rather than inventing a per-beam conversion.

This gives the local-ring screen two distinct external references: the 100 TeV / 16 T CDR stretch case and the 85 TeV / 14 T updated study case.
