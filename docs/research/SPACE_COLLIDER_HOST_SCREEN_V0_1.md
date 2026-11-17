# Space Collider Host Screen v0.1

**Status:** first-order analytical host/body screen; not a collider design, site qualification or feasibility finding  
**Parent:** [SPACE-COLLIDER-001 #3953](https://github.com/Luminous-Dynamics/symtropy/issues/3953)  
**Host-trade work:** [SPACE-COLLIDER-HOST-TRADE-001 #5529](https://github.com/Luminous-Dynamics/symtropy/issues/5529)  
**Existing narrow analytical reference:** [PR #3962](https://github.com/Luminous-Dynamics/symtropy/pull/3962)

## Executive finding

The Moon is the strongest *first body-host research case*, not a proven best collider site. It has a published, peer-reviewed great-circle collider study, is comparatively close to Earth, and is a natural starting point for evaluating surface/subsurface construction and cislunar industrialization.

That surface-hosted Moon collider is **not an orbital ring**. A free-flying orbital accelerator constellation is a separate architecture, as is an accelerator carried by a mechanically supported orbital-ring system.

A first-order body-great-circle calculation gives higher rigidity-only energy ceilings for Mars and Earth than for the Moon. But at fixed proton beam current and dipole field, synchrotron-radiation power grows approximately as the cube of bending radius. The larger-body energy frontier therefore comes with a severe power penalty. Ranking hosts by radius or ideal centre-of-mass energy alone is physically misleading.

Working recommendation:
- Start body-hosted site and construction research with the Moon.
- Keep a smaller, local Moon-ring architecture distinct from a great-circle lunar machine.
- Keep free-flying upper-LEO/MEO/GEO configurations as separate orbital accelerator alternatives.
- Treat Mars and Jovian moons as longer-horizon trade-study cases, Ceres as an ISRU/resource comparator, and Phobos/Deimos as compact-body/low-energy cases.
- Do not add an orbital-ring structure unless a matched study demonstrates a system-level advantage over an independent accelerator constellation or body-hosted ring.

## 1. Three different circular infrastructures

### Body-anchored accelerator

The magnets, vacuum/beam enclosure, RF and detector systems are mounted on a surface or placed in a tunnel/cut-and-cover route. The 2022 Beacham–Zimmermann concept follows a lunar great circle of about 11,000 km. This is a surface/subsurface collider proposal, not a mass-stream orbital ring.

### Free-flying orbital accelerator

Accelerator cells are distributed around a common orbit. Each cell must maintain the beamline's position, attitude, field quality, timing and relative alignment while orbital perturbations act on the constellation. The May 2026 space-collider preprint explores this architecture. It is a preprint and its system assumptions remain to be independently challenged.

### Orbital-ring-supported accelerator

A distinct orbital-ring structure or moving mass-stream system carries some or all accelerator components. Its support, dynamics, loads, control, orbital stability, common-cause failures and maintenance are additional subsystems. The ring does not automatically solve accelerator beam optics, luminosity, RF, injection, detector, magnet or beam-abort requirements.

A fourth option is a **local engineered accelerator ring on a body**. Its circumference is selected by accelerator requirements; it does not need to follow the body's full circumference.

## 2. Rigidity-only body-great-circle screen

For a symmetric head-on proton collider, using an idealized circular bending path:

`p_{beam}[GeV/c] \approx 0.299792458\,B[T]\,\rho[m]`

`E_{CM}[GeV] \approx 2\times 0.299792458\,B[T]\,f_{dip}\,R[m]`

Here:
- `B` is the assumed dipole field;
- `f_dip` is the global fraction of the full path occupied by bending dipoles;
- `R` is the assumed body mean radius, used as the bending radius for a notional great-circle route;
- the factor of two is for equal-energy, head-on proton beams.

The screen uses the lunar paper's optimistic `B = 20 T` and `f_dip = 0.67`. The radius inputs are published bulk/mean-radius values: NASA/JPL sources for Earth, Mars, Moon, Ceres, Phobos, Deimos and the Galilean moons. The values below are calculations from those inputs, not observed accelerator performance.

| Host | Mean radius (km) | Great-circle circumference (km) | Idealized rigidity-only E_CM |
|---|---:|---:|---:|
| Earth | 6,371.0 | 40,030 | 51.2 PeV |
| Mars | 3,389.5 | 21,297 | 27.2 PeV |
| Ganymede | 2,631.2 | 16,532 | 21.1 PeV |
| Callisto | 2,410.3 | 15,144 | 19.4 PeV |
| Io | 1,821.5 | 11,445 | 14.6 PeV |
| Moon | 1,737.4 | 10,916 | 14.0 PeV |
| Europa | 1,560.8 | 9,807 | 12.5 PeV |
| Ceres | 476.0 | 2,991 | 3.82 PeV |
| Phobos | 11.08 | 69.6 | 89.0 TeV |
| Deimos | 6.2 | 39.0 | 49.8 TeV |

**Interpretation limits**

- These numbers are idealized rigidity ceilings for a body-great-circle proton path. They are not achievable-machine predictions.
- They assume a 20 T bending field and a global 0.67 dipole fill factor. The 20 T field is an optimistic research assumption, not an operational magnet supply chain.
- The table has no lattice, focusing, chromatic correction, beam–beam limit, injection, RF, detector, luminosity, beam lifetime, aperture, field quality, alignment, power or thermal design.
- Phobos and Deimos are irregular bodies. A spherical mean-radius circle is only a scale illustration; a real route needs shape models, local gravity and anchoring mechanics.
- A local ring on the Moon or Mars can have a much smaller radius than the host body. Body radius is not a compulsory machine radius.

## 3. Why "largest body wins" fails

For proton synchrotron radiation at fixed beam current and fixed dipole field, the leading scaling is approximately:

`P_{SR}\propto I_{beam}\,B^4\,\rho^3`

This is a screening relation, not a substitute for a lattice- and beam-specific radiation calculation.

The lunar collider paper estimates about 8.5 TW of synchrotron-radiation replacement power for its illustrative 14 PeV CM machine. Using the same fixed-current, fixed-field scaling only as a comparison:
- Mars' mean-radius track would have roughly `(3389.5 / 1737.4)^3 ≈ 7.4` times the Moon's radiation power, or about 63 TW.
- Earth's mean-radius track would have roughly `(6371 / 1737.4)^3 ≈ 49` times the Moon's radiation power, or about 420 TW.

These are **derived scaling illustrations**, not new machine simulations. They inherit the lunar paper's beam-current and accelerator assumptions and do not include site power, refrigeration, RF efficiency, duty cycle, beam lifetime or engineering constraints. The point is that a higher rigidity ceiling can be overwhelmed by its radiation-power cost.

The Moon paper itself reports about 8.5 TW, around an 18-minute beam lifetime and about 550,000 14 m dipoles for its tentative design. It also reports approximately one million events per bunch crossing. These expose central challenges in power, top-up injection, magnet manufacture, detector segmentation and data acquisition. The paper is a long-term concept study, not a build-ready design.

## 4. Host-by-host interpretation

### Moon — first body-host research candidate

The Moon has a specific published great-circle collider concept and a well-developed cislunar logistics/ISRU research ecosystem. It is therefore the right first case for comparing:
- mare cut-and-cover;
- subsurface tunnels;
- surface or elevated beamline corridors;
- local, FCC-scale machine rings;
- a hypothetical full great-circle machine.

Its challenges include route topography, excavation and ground support, dust, thermal cycling, micrometeoroids, radiation, local resource assay, large-scale conductor production, power storage, RF replacement power and radiative heat rejection. A surface vacuum argument is not by itself proof that an unshielded/open magnet lattice can run with acceptable beam-gas backgrounds, charging and contamination.

### Mars — higher theoretical rigidity, much harder site/system trade

Mars' larger radius produces a higher body-great-circle rigidity screen than the Moon. It also implies a longer accelerator route and much larger synchrotron-radiation power at matched field/current. Its atmosphere, dust, weather, terrain, geotechnics, logistics and solar/energy system must be included. The accelerator beam enclosure's vacuum requirement must be modelled separately from the ambient Martian atmosphere.

### Ceres — resource and industrialization comparator

Ceres has a mean radius of 476 km and an average solar distance of 2.8 AU, and NASA describes water-bearing materials. It is relevant to resource processing and off-Earth manufacturing research, but its body-great-circle rigidity ceiling is lower than the Moon's. Solar irradiance at 2.8 AU is about 1/7.8 of Earth's before local geometry and shadowing; power and heat rejection therefore need careful treatment.

Water, hydrated salts or other detected materials must not be promoted to accelerator-grade conductor, dielectric, coolant or structural feedstock without assay, extraction, refining, process yield and qualification evidence.

### Phobos and Deimos — compact-body cases, not PeV great-circle hosts

Their small radii yield sub-100-TeV rigidity-only values under the same optimistic field/fill assumption. Their irregular shapes, low gravity and local anchoring mechanics make a continuous body-attached ring a distinct structural problem. A free-flying satellite orbit around Phobos must not be assumed to be stable; quasi-satellite trajectories require orbit maintenance. These bodies remain useful for small industrial demonstrations and compact low-energy infrastructure studies, not as first choices for a PeV-class collider.

### Jovian moons — large geometry, severe environmental and logistics questions

Ganymede and Callisto provide larger body radii than the Moon, and therefore higher notional rigidity ceilings under identical assumptions. Their orbital setting around Jupiter introduces radiation, charging, communications, logistics and operational constraints that are unlike Earth–Moon space. Io and Europa need particularly careful radiation and surface-environment analysis. These bodies should remain a comparison stratum, not be ranked by radius alone.

### Free-flying Earth orbit — strongest separate orbital-accelerator study

The 2026 preprint compares upper LEO, MEO and GEO. It identifies the 47,000 km LEO-scale circumference's approximately 6.5 Hz revolution frequency, compared with about 11,245 Hz for the LHC, as a central luminosity penalty. It then argues that beam current, bunch count and radiation damping could compensate part of the loss, while listing unresolved challenges in final-focus optics, beam–beam tune shift, detector design, event rate, constellation replenishment and lifetime.

The preprint's claimed ~140x Lorentz-factor compensation at "1 PeV" is not safe to adopt as a fixed input: PR #3962 flags the centre-of-mass/per-beam energy convention ambiguity and computes 71.43x if both energies are interpreted as centre-of-mass values, versus 142.86x if 1 PeV is per beam. Both interpretations need to remain explicit until the authors clarify the convention.

The preprint favours upper LEO as a first-generation orbit under its assumptions. That is a hypothesis to reproduce, not an independently validated orbit selection.

## 5. What "best" means

There is no defensible single winner without a defined scientific programme and operating envelope.

- **Best first body-host research case:** Moon, because the 2022 concept gives a concrete reference and cislunar operations are the nearest body-hosted context.
- **Highest idealized surface rigidity ceiling among the screened rocky planets:** Earth, followed by Mars—but power, geology, environment and infrastructure make that a poor standalone ranking.
- **Interesting outer-system host comparison:** Callisto versus Ganymede, with radiation, power, resource and logistics included.
- **Interesting resource-industrialization comparison:** Ceres.
- **Compact-body systems study:** Phobos/Deimos, without assuming a free satellite orbital ring.
- **Best current orbital-accelerator study starting point:** upper LEO as proposed by the 2026 preprint, compared against MEO and GEO under equal power, luminosity, formation, radiation and lifetime assumptions.
- **Best actual machine:** unknown. It must be selected by useful integrated luminosity, physics case, power, mass, reliability, detector capability, maintenance and lifecycle cost—not rigidity alone.

A 100 km class collider on the Moon is a different candidate from an 11,000 km lunar great-circle collider. It should be included because the accelerator circumference is a design variable, not a fact imposed by the host body.

## 6. Benchmark progression

The next studies should proceed in this order:

1. Freeze source-pinned body radii, shapes, gravity, rotation, topography, solar flux, radiation and ephemeris inputs.
2. Reproduce the 2022 lunar collider paper's arithmetic and machine parameters independently, including per-beam/centre-of-mass conventions, RF replacement power, luminosity, beam lifetime and pile-up.
3. Reproduce the 2026 orbital preprint's orbit and luminosity trade under both energy conventions.
4. Add local Moon/Mars machine rings and the CERN FCC-hh 100 TeV/97.75 km/16 T conceptual baseline.
5. Introduce proton/electron/muon species as separate physics profiles; do not reuse proton synchrotron-radiation formulas without correct particle-mass scaling. For muons, include decay, neutrino radiation, cooling and rapid injection.
6. Run matched-fidelity lattice/beam-dynamics studies with established accelerator tools and specialist review.
7. Only then compare power system, magnet production, detector, assembly/maintenance and host infrastructure costs.
8. Keep surface collider, free-flying orbital collider and orbital-ring-supported collider as separate architecture rows.

Report energy, luminosity, power, event/pile-up, beam lifetime, mass, maintenance and lifecycle separately. No single composite "collider score" should hide an unacceptable detector, radiation or power requirement.

## 7. Source and status notes

- Beacham & Zimmermann, "A very high energy hadron collider on the Moon," *New Journal of Physics* 24 (2022) 023029, https://doi.org/10.1088/1367-2630/ac4921. Peer-reviewed long-term concept paper.
- Danchev et al., "The Case for Space-Based Particle Colliders," arXiv:2605.08239 (2026). Preprint, not peer-reviewed physical qualification.
- CERN, *The hadron collider FCC-hh: Extended conceptual design report*, CERN-2025-006, https://doi.org/10.23731/CYRM-2025-006. Use as a proposed accelerator reference, not an operational collider.
- JPL Planetary Satellite Physical Parameters: https://ssd.jpl.nasa.gov/sats/phys_par/.
- NASA Mars Fact Sheet: https://nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html.
- NASA Jovian Satellite Fact Sheet: https://nssdc.gsfc.nasa.gov/planetary/factsheet/joviansatfact.html.
- NASA Ceres Facts: https://science.nasa.gov/dwarf-planets/ceres/facts/.
- ESA Phobos System Requirements Review / CDF study: https://sci.esa.int/documents/34923/36148/1567260187106-PhobosSR_v2_Releasable.pdf.

## Claim ceiling

This note is a reproducible first-order host-body geometry and rigidity screen. It is not an accelerator lattice, validated beam-dynamics result, luminosity prediction, magnet/cryogenic design, site approval, environmental assessment, orbital-ring qualification, construction estimate or feasibility finding.