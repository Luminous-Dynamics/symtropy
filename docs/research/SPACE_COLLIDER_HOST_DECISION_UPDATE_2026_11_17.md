# Space Collider Host Decision Update — 17 November 2026

**Status:** analytical decision update; no accelerator host or site is selected or qualified.  
**Base screen:** [SPACE_COLLIDER_HOST_SCREEN_V0_1.md](SPACE_COLLIDER_HOST_SCREEN_V0_1.md)  
**Owners:** [#3953](https://github.com/Luminous-Dynamics/symtropy/issues/3953), [#5529](https://github.com/Luminous-Dynamics/symtropy/issues/5529), [#5583](https://github.com/Luminous-Dynamics/symtropy/issues/5583), [#5907](https://github.com/Luminous-Dynamics/symtropy/issues/5907).  
**Existing analytical reference:** [PR #3962](https://github.com/Luminous-Dynamics/symtropy/pull/3962).

## Decision in one paragraph

We have **not found one best body**. The evidence supports a mission-conditioned shortlist:

- **Earth** remains the operating-facility baseline because it has the existing accelerator, power, industrial, scientific and maintenance ecosystem. It is the correct near-term reference, not proof that Earth is optimal for every future energy frontier.
- **Moon** remains the first body-host research case because it is comparatively accessible from Earth and has a peer-reviewed lunar-collider reference. It is not a selected site or a demonstrated best machine host.
- **Callisto** is the lead outer-system host comparison because it is farther from Jupiter than Europa, Ganymede and Io and experiences a materially different, generally less intense Jovian particle environment. It is still an extremely remote, low-solar-flux, cryogenic logistics case—not a near-term choice.
- **Mars** has a larger body-radius rigidity ceiling than the Moon, but its fixed-current / fixed-field whole-body proton-ring synchrotron-power scaling is much worse. This makes it a useful stress-test, not a current preferred host.
- **Ceres** remains a resource-industrialization comparison, not a high-rigidity leader.
- **Ganymede** is a useful Callisto contrast: slightly larger, with an intrinsic magnetic field, but its radiation environment is not interchangeable with Callisto's.
- **Europa and Io** should remain hostile-environment controls rather than lead host candidates.

The correct near-term machine may remain on Earth; the correct first off-Earth body research case is still the Moon; the best actual accelerator architecture remains unknown.

## What changes the answer: particle species and machine circumference

A body's mean radius is not the accelerator's required bending radius. A body can host a local ring, tunnel, linear accelerator, distributed system or no accelerator at all. The previous great-circle table is a geometry/rigidity screen only.

### Proton collider: the lunar reference is a severe power challenge

Beacham and Zimmermann's peer-reviewed lunar concept estimates about 14 PeV centre-of-mass energy for an approximately 11,000 km lunar great-circle ring using an optimistic 20 T dipole field. Its illustrative design includes roughly 8.5 TW synchrotron-radiation replacement power, an approximately 18-minute beam lifetime and about 550,000 14 m dipoles. These are source-paper design estimates, not measured performance or an approved engineering design.

Under the simple fixed-beam-current / fixed-field scaling already used in v0.1:

\[
P_{SR}\propto I B^4 \rho^3
\]

The Mars-to-Moon mean-radius ratio is approximately 1.950; its cube is approximately **7.42**. Therefore, a Mars great-circle proton ring at the same current and field would carry roughly 7.42 times the lunar reference's synchrotron-radiation power burden before any other site or lattice differences. This is a screening ratio, not a Mars machine simulation. Earth's analogous radius-cubed ratio is approximately 49.3 relative to the Moon.

That is why the higher nominal rigidity ceiling on a larger body cannot be treated as a win.

### Muon collider: a lunar great circle is not the 2026 industry reference

The 5 November 2026 National Lab Accelerator Study Group reference is a **10 TeV centre-of-mass muon collider** with an 11.4 km circumference, 14 T arc peak field, 14.4 MW beam power, and 1,363-turn luminosity lifetime. The same report identifies roughly 500 W/m beam-induced cold-mass deposition and off-site neutrino radiation as significant system constraints. The integrated sequence from muon production through cooling, acceleration and collisions has not yet been demonstrated.

The existing Symtropy muon-scaling reference (#5583, via PR #3962) has already exposed two very different Moon-scale extrapolations:

- **Hold beam energy at 5 TeV per beam:** the roughly 11,000 km ring gives only about 2.8 particle-decay e-fold turns (about 1.4 two-beam luminosity e-fold turns) under the declared simple decay model.
- **Hold 14 T and the source-implied dipole fill:** the beam energy scales to roughly 4.82 PeV per beam, but the classical synchrotron screen gives about 4.73 beam energies lost per turn and a formal replacement scale near 90 TW per beam. The report marks this as no fixed-energy circular equilibrium under that simplified model.

These are arithmetic / scaling falsification cases, not full muon lattice, RF, transport or neutrino simulations. They are enough to reject the slogan that 'muons solve synchrotron radiation' as a sufficient argument for a planetary-scale ring. A local ring and a staged acceleration chain must be evaluated as a different architecture.

## Body-host implications

| Host | Current role in the comparison | Strongest grounded advantage | Major unresolved penalty |
|---|---|---|---|
| Earth | Operating baseline | Existing accelerator, grid, workforce, supply chain, labs and repair infrastructure | Site, geology, land use, environmental permitting, cryogenic and power limits |
| Moon | First body-host research case | Proximity to Earth; published lunar-collider concept; local surface/subsurface routes; reduced gravity | Power storage / firm generation, dust, long thermal cycles, radiation, route construction, radiative heat rejection |
| Mars | Stress-test / longer-horizon alternative | Larger body radius and 0.431 Earth solar irradiance is still usable with large arrays | Whole-body proton-ring SR penalty; atmosphere and dust; long logistics chain; less solar flux than Moon/Earth |
| Ceres | Resource-industrialization comparator | Water-bearing materials and short rotation period | 2.8 AU solar distance (about 0.128 Earth irradiance); small body-radius rigidity ceiling; resource processing unqualified |
| Callisto | Lead Jovian-moon comparison | Farther from Jupiter; NASA/JPL particle-energy-flux models distinguish it from the intense Europa environment | About 5.2 AU from Sun (about 0.037 Earth irradiance); remote transport, communications, cryogenic and repair logistics |
| Ganymede | Callisto comparator | Slightly larger body radius; intrinsic magnetic field | Different particle environment; magnetic shielding does not imply zero surface dose |
| Europa / Io | Environmental controls | Scientific / resource interest, not host suitability | Severe Jovian particle environment and, for Io, strong geological activity |
| Upper LEO / MEO / GEO | Separate free-flying architecture | No body-tunnel excavation; orbital vacuum | Formation metrology/control, orbit perturbations, debris/radiation, low revolution frequency and luminosity closure |

### Environmental details that materially affect the ranking

- NASA's lunar south-pole environment reference shows strongly location-dependent illumination and thermal conditions: some high terrain can receive extended sunlight, while nearby permanently shadowed areas can be extremely cold. The Moon's surface radiation is not negligible; a Nature Astronomy model estimates galactic-cosmic-ray exposure at about 66% of interplanetary space on the surface, with substantial reduction at roughly 3 m regolith depth. This makes buried infrastructure and shielding part of the host design, not optional decoration.
- NASA's Mars fact sheet gives a solar irradiance of 586.2 W/m² versus 1,361 W/m² at Earth (about 0.431×). Atmospheric and dust effects must be modelled separately from the beamline vacuum enclosure.
- NASA gives Ceres a mean radius of 476 km and an average solar distance of 2.8 AU. That corresponds to about 0.128× Earth's solar irradiance before local geometry and shadowing. Water-bearing materials are a resource hypothesis, not qualified accelerator conductor or cryogenic feedstock.
- Callisto orbits in the outer part of the Jovian moon system and is about 5.2 AU from the Sun. A NASA/JPL energetic-particle study reports incident energy flux at Europa around \(8\times10^{10}\) keV cm\(^{-2}\) s\(^{-1}\), compared with roughly \(2\text{–}3\times10^8\) at Callisto and Ganymede's equator. This is a particle-energy-flux comparison, not a direct human dose or a radiation-safety certification. Local surface geometry, particle species, shielding depth and time-dependent magnetospheric conditions still matter.

## State-of-the-art comparison

| Reference | Published / programme parameters | What it establishes | What it does not establish |
|---|---|---|---|
| LHC / HL-LHC | Operating proton-collider reference; use exact current source revision for each metric | Demonstrated machine operation and detector integration | A 100 TeV or PeV collider |
| FCC-hh | 100 TeV CM, 97.75 km circumference, 16 T conceptual baseline; about 2.4 MW synchrotron power per beam | Strongest current future-hadron-collider systems comparator | Built or operational hardware |
| 2025 FCC magnet R&D | 14 T Nb\(_3\)Sn path and 20 T HTS research direction | Magnet technology development and scale-up challenges are explicit | 20 T accelerator-grade magnets available at collider production scale |
| NLASG 2026 muon reference | 10 TeV CM, 11.4 km, 14 T, 14.4 MW beam power; 500 W/m cold-mass deposition target | A detailed, current systems-level R&D reference | Integrated muon production-to-collision operation |
| Lunar CCM | 14 PeV CM source-paper concept; optimistic 20 T and ~8.5 TW SR replacement | An ambitious body-hosted proton-collider study | Demonstrated beam stability, magnet manufacturing, power, detector or construction feasibility |
| Space-collider preprint (2026) | Free-flying orbital constellation concepts | A useful architecture hypothesis and challenge set | Validated formation control, luminosity, beam stability or integrated operation |

Do not call a 14–20 T concept 'state of the art' in the same sense as an operating collider magnet. CERN reports a 14.1 T accelerator dipole demonstration at 4.5 K; 16 T Nb\(_3\)Sn and 20 T HTS work remains a staged development programme with field quality, aperture, manufacturing yield and cost still to qualify.

## New paradigm: choose the machine, power plant and body together

The next-generation design variable is not just body radius. It is the joint tuple:

\[
(\text{species},\; C,\; B,\; \mathcal L_{int},\; P_{firm},\; P_{SR},\; Q_{cold},\; \text{detector},\; \text{host},\; \text{repair})
\]

A strong design process should:

1. **Separate host from lattice scale.** Compare 50–150 km local rings on Earth and Moon before treating a 10,000 km great circle as the natural machine.
2. **Co-design beam schedule and utilities.** A pulsed, staged machine can be paired with storage and generation profiles, but any schedule-induced luminosity loss must be included.
3. **Treat heat as a first-class design boundary.** In vacuum, radiator capacity depends on temperature, emissivity, view factor, solar/planetary loading, surface degradation and mass. 'Space is cold' is not an engineering calculation.
4. **Select architecture by science delivered.** Optimize integrated luminosity, accepted events, beam quality and uptime per delivered MW and lifecycle MWh—not peak centre-of-mass energy alone.
5. **Use existing specialist solvers.** Compare lattice and collective beam dynamics with MAD-X / Xsuite / SixTrack / COMBI and experiment-grade tools; use Geant4 / FLUKA-class references for radiation and heat transport. Symtropy should own reference cases, matched interfaces, independent checks and evidence receipts rather than duplicate the specialist engines.
6. **Make reliability and repairability explicit.** Report component replacement, spare mass, repair latency, alignment recovery and N-1 utility behavior.
7. **Preserve multi-objective trade-offs.** Hard safety, stability, detector, power and thermal gates cannot be compensated by a higher energy number.

## Benchmark targets

Targets below are proposed programme goals, not achieved results.

| Benchmark layer | Acceptance target | Stretch target |
|---|---|---|
| Source / geometry | 100% source attribution, pinned units, body constants and input digests | Independent second-source agreement within declared uncertainty |
| Arithmetic / conservation | 100% known-answer and negative-control coverage; power and energy ledgers close within predeclared tolerances | Independent implementation reproduces every held-out reference case |
| Accelerator physics | Matched energy, lattice, beam species, field and optics; compare against established specialist tools | Demonstrate robust gains on a preregistered held-out parameter set, with beam quality and luminosity preserved |
| Luminosity / detector | Report peak and integrated luminosity, pile-up, detector acceptance and rare-signal yield separately | Increase accepted physics yield per MW without increasing unsafe backgrounds or degrading beam quality |
| Power / thermal | Full load map, firm power, storage, cryogenic overhead and radiative rejection; no missing heat term | Higher integrated luminosity per lifecycle MWh and lower radiator / power-system mass at equal operating envelope |
| Site / reliability | Exact site horizons, radiation, dust, thermal cycles, outage, repair and N-1 tests | Demonstrate robust Pareto improvement across held-out site profiles |
| Evidence / reproducibility | Exact-head code, toolchain, input digests, deterministic receipts, independent verification | Independently repeated results across two implementations and a physical component correlation |

A result is better than the state of the art only if the comparison is matched and independently reproducible. A higher ideal rigidity ceiling, a prettier simulation or a new architecture label is not sufficient.

## Work opened

[SPACE-COLLIDER-HOST-THERMAL-001 #5907](https://github.com/Luminous-Dynamics/symtropy/issues/5907) now owns the missing host-side firm-power, storage, cryogenic utility and heat-rejection envelope. It consumes the beam-side outputs from #5567/#5583/#5604/#5572 and feeds the host comparison in #5529; it does not duplicate them.

## References

- Beacham & Zimmermann, *A very high energy hadron collider on the Moon*, New Journal of Physics 24 (2022) 023029: https://doi.org/10.1088/1367-2630/ac4921
- CERN, *FCC-hh Extended Conceptual Design Report*, CERN-2025-006: https://doi.org/10.23731/CYRM-2025-006
- CERN, FCC magnet design programme: https://fcc.web.cern.ch/magnet-design
- National Lab Accelerator Study Group, *Report of the National Lab Accelerator Study Group for a Muon Collider*, 5 November 2026: https://arxiv.org/abs/2611.06680
- Danchev et al., *The Case for Space-Based Particle Colliders*, 2026 preprint: https://arxiv.org/abs/2605.08239
- NASA, Moon south-pole environment: https://www.nasa.gov/reference/moonbase-environment/
- Dobynde & Guo, *Guidelines for radiation-safe human activities on the Moon*, Nature Astronomy (2024): https://www.nature.com/articles/s41550-024-02287-8
- NASA, Mars Fact Sheet: https://nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html
- NASA, Ceres facts: https://science.nasa.gov/dwarf-planets/ceres/facts/
- JPL, Planetary Satellite Physical Parameters: https://ssd.jpl.nasa.gov/sats/phys_par/
- NASA/JPL, *Energetic Ion and Electron Irradiation of the Icy Galilean Satellites*: https://ntrs.nasa.gov/citations/20010125134

## Claim ceiling

This update is an evidence-bounded research prioritization. It does not select a construction site, validate a collider lattice, predict achievable luminosity, qualify a magnet / power / cryogenic / radiator system, establish radiation safety or claim that any off-Earth accelerator is feasible.


## Addendum — local engineered rings change the host question (17 November 2026)

The great-circle comparison is not sufficient to select a body host. The accelerator circumference is a design variable; a local ring on the Moon does not have to follow the Moon's 10,916 km great circle.

A first-order **local-ring comparison has now been opened in [#5924](https://github.com/Luminous-Dynamics/symtropy/issues/5924) and implemented in the existing analytical reference on [PR #3962](https://github.com/Luminous-Dynamics/symtropy/pull/3962)**. It covers 50 km, 100 km and 150 km circumferences at 8.33 T, 14 T, 16 T and 20 T, with a declared 0.67 dipole fill fraction. This is deliberately an in-place extension of the existing space-collider reference, not a second calculator.

### Key matched comparison

At 97.75 km, 16 T and 0.67 fill:
- the simplified proton rigidity screen gives **99.9959 TeV CM**, almost exactly the 100 TeV FCC-hh CDR geometry reference;
- with a 0.5 A proton reference current, the isomagnetic synchrotron screen gives **2.3339 MW/beam**, versus the FCC-hh source reference of approximately 2.4 MW/beam;
- both residuals are retained without fitting or calibration.

At 100 km / 16 T / 0.67 fill, the screen gives **102.2976 TeV CM**. This makes a local 100 km lunar ring a more decision-relevant comparison to FCC-hh than the 11,000 km lunar great-circle concept. It does not establish that a lunar site, magnet system, lattice, detector, power plant or collider could be built.

### Species changes the local-ring trade

Under the same 100 km / 16 T / 0.67 geometry:
- the simplified muon decay model gives about **3,188 particle-decay e-fold turns**, with about **1,594 two-beam luminosity decay e-fold turns**;
- reusing the NLASG 1.8e12-muon bunch and 5 Hz repetition profile gives approximately **73.75 MW/beam injection power**;
- the formal isomagnetic synchrotron estimate is approximately **142.9 MW/beam**, or about **1.94 times** the injection-power reference.

This is a continuous repetition / exponential decay scaling calculation. It is not a bunch-resolved operating point or a credible power forecast. The executable now emits an explicit status when formal synchrotron replacement power exceeds the declared injection-power profile. The detector, decay-product transport, neutrino-dose, RF, magnet, and host utility questions remain separate owners.

### Current industry frontier and the real improvement target

- CERN's FCC programme reports a **14.1 T accelerator dipole demonstration at 4.5 K**. Its programme is developing Nb3Sn toward 16 T and investigating HTS inserts/demonstrators toward 20 T; field quality in a 50 mm aperture, protection, and large-scale manufacture remain explicit challenges. A 20 T input in our screen is therefore a stretch sensitivity case, not an available production field. Reference: https://fcc.web.cern.ch/magnet-design
- The November 5, 2026 NLASG muon-collider report states that the integrated production–cooling–acceleration–collision chain has not been demonstrated. It argues for a systems-integrated R&D paradigm in which simulation, component tests, systems engineering and experiments retire full-machine risks together. The report's 10 TeV collider is the relevant current muon reference, not our PeV extrapolation. Reference: https://arxiv.org/abs/2611.06680
- Beam-driven plasma wakefields can reach gradients up to **100 GV/m**, but the September 2026 accepted Reviews of Modern Physics review identifies efficiency and beam-quality preservation as challenges. The 2026 PDG accelerator review notes that plasma-based collider luminosity remains orders of magnitude below conventional collider projections and highlights positron focusing / beam transport problems. High gradient alone does not make a collider. References: https://doi.org/10.1103/2lw2-szhs and https://ccwww.kek.jp/pdg/2026/reviews/rpp2026-rev-accel-phys-colliders.pdf

### New programme direction

Compare machines using **accepted scientific output per delivered MW, installed mass and lifecycle effort**, under matched beam quality, detector acceptance, reliability and operating schedule. The near-term comparator should include:
1. operating Earth facilities as the demonstrated baseline;
2. FCC-hh as the future conventional 100 TeV hadron reference;
3. a 50–150 km local Moon ring as a body-hosted geometry/power/thermal candidate;
4. the 10 TeV muon programme as an emerging lepton architecture;
5. plasma-wakefield acceleration as a staged-booster / hybrid research route, conditional on finite-emittance input and output, efficiency, positron/driver constraints and integrated luminosity.

Do not average these into a single scalar winner. First reject concepts that fail hard beam-stability, detector, machine-protection, radiation, power or thermal gates; then compare Pareto trade-offs among feasible candidates.

### Validation state

The new local-ring module and deterministic JSON / workflow assertions are committed in PR #3962. The latest exact head at this update is expected to be checked by the PR-triggered focused and repository CI workflows; queued work is not a pass. The numerical results above are independently recomputable from the declared rigidity and classical isomagnetic equations, but the Rust executable's test/CI result must be reported only after exact-head completion.
