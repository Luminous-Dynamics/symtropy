# Orbital Access Architecture and Terminal Geometry Benchmark v0.1

**Status:** concept/research benchmark; synthetic-first; no flight or infrastructure qualification
**Tracking:** [ORBITAL-ACCESS-001 #3202](https://github.com/Luminous-Dynamics/symtropy/issues/3202)
**Composition:** [SPACE-CAPABILITY-LEDGER-001 #1972](https://github.com/Luminous-Dynamics/symtropy/issues/1972), [reference frames #98](https://github.com/Luminous-Dynamics/symtropy/issues/98), [conserved interbody transit #103](https://github.com/Luminous-Dynamics/symtropy/issues/103), [independent verifier #1737](https://github.com/Luminous-Dynamics/symtropy/issues/1737), Sol-Atlas [spaceport geography #393](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/393)

## 1. Decision problem

Compare distinct Earth-to-orbit architectures against the actual operational baseline and derive their compatible terminal/access geography. The output is not a list of locations with the highest equatorial score. It is a set of architecture-specific investigation envelopes conditioned on a target service, orbital state, geometry, physical model and terrestrial constraints.

## 2. Architecture taxonomy

1. **Chemical/reusable rocket:** current operational baseline, selected by vehicle capability, launch site, target orbit, launch azimuth, downrange safety and range availability.
2. **Air-launch / launch assist:** aircraft/base/airspace/release-envelope architecture; does not remove orbit-insertion delta-v.
3. **Classical geostationary space elevator:** equatorial Earth anchor and a tether extending through GEO to a counterweight beyond GEO; a different architecture from a low-orbit ring.
4. **Full orbital-ring system (ORS):** concept in which a moving mass/ring in low Earth orbit supports skyhook/ladder structures; architecture, orbital plane, stability and support dynamics must be explicitly specified.
5. **Partial orbital-ring system (PORS):** bounded segment with a pair of ground terminals/endpoints and its own moving mass stream, cable, support and control system.
6. **Orbital skyhook / momentum-exchange tether:** moving orbital tether with capture/release states; its orbit and momentum/energy budget are distinct from a stationary elevator.
7. **Launch loop:** ground-supported elevated accelerator/mass-stream structure with its own endpoints and dynamic stability; not an orbital ring.
8. **Ground freight MagRail/maglev:** terrestrial logistics mode only; no orbital launch authority or physics is implied.

## 3. Why equator-only site ranking is wrong

At latitude phi, the eastward surface-rotation speed is approximately omega times Earth-radius times cos(phi). It is largest at the equator. NASA gives an equatorial pad speed above 1,650 km/h and explains that the advantage depends on the target orbit; high-inclination missions do not receive the same eastward benefit. Source: https://science.nasa.gov/learn/basics-of-space-flight/chapter14-1/

For a simplified direct insertion at geodetic latitude phi, let A be launch azimuth measured clockwise from local north in the Earth-fixed frame, V the vehicle's horizontal speed relative to the rotating surface at insertion, and v_rot = omega * R(phi) * cos(phi) the local eastward surface speed. Resolve the inertial horizontal velocity as v_N = V * cos(A) and v_E = V * sin(A) + v_rot. Under a spherical-Earth, horizontal-at-insertion approximation, cos(i) = cos(phi) * v_E / sqrt(v_N^2 + v_E^2). This formulation explicitly includes Earth rotation; it is a reference fixture, not a vehicle performance model. A due-east launch yields i approximately |phi|, while a high-inclination/polar launch receives little or no useful eastward rotational assistance. Real ascent steering, finite insertion flight-path angle, non-spherical gravity, azimuth constraints, launch windows and plane changes require the qualified mission model.

A concrete synthetic check uses R = 6,378,137 m, omega = 7.2921150e-5 rad/s and V = 7,800 m/s: at the equator with Earth-fixed azimuth A = 0 degrees (due north), v_rot = 465.1010849 m/s and the idealized inclination is 86.587588 degrees, not 90 degrees. An azimuth of approximately -3.418480 degrees (slightly west of north) cancels that eastward inertial velocity and produces the 90-degree polar-plane fixture. For a due-east insertion, inclination equals the absolute site latitude in this spherical idealization. These values are regression fixtures only, not a recommended launch azimuth.

Thus, a near-equatorial site can be advantageous for some low-inclination/eastward missions but is not intrinsically the best location for polar or sun-synchronous traffic. An east-facing coastline only matters if the actual target orbit and permissible downrange corridor align.

## 4. Space elevator material and deployment gate

A 2023 Acta Astronautica review surveys carbon nanotube, graphene-based and hexagonal-boron-nitride candidates. It describes laboratory samples with promising specific-strength properties but reports that no candidate material existed in sufficient quantity for the required very long tether, and identifies material continuity/manufacturing at industrial scale as a central challenge. Source: https://doi.org/10.1016/j.actaastro.2023.04.008

Do not convert a record tensile-strength coupon into a qualified 100,000 km-class tether. Material quality, defects, joins, fatigue, radiation/atomic oxygen, impact protection, thermal environment, manufacturing rate, inspection, repair, climber dynamics, counterweight and deployment all require independent qualification. Space-elevator site ranking is not useful until the enabling material/system envelope exists.

## 5. Orbital ring and skyhook are not the space elevator

Paul Birch's 1982 JBIS paper is a preliminary conceptual reference for orbital ring systems and Jacob's Ladders. It discusses moving ring systems in low Earth orbit, skyhook/ladder interfaces and partial-ring variants with two ground endpoints. Its idealized discussion permits orbital planes beyond the equatorial plane. It is not an engineering demonstration, flight test, qualified design or proof of material/control feasibility. Source: https://nss.org/wp-content/uploads/Orbital-Rings.pdf

Consequences:
- do not constrain an ORS terminal search to the equator by analogy with a classical geostationary elevator;
- derive terminal geography from the selected ring plane, orbit, dynamics, skyhook/ladder motion and Earth rotation;
- select partial-ring endpoints as a coupled pair under one dynamic model, not as two independent nearest-port locations;
- treat skyhook capture/release as momentum exchange requiring a replenishment/energy strategy;
- keep ring, tether, launch loop and rocket architectures in separate model and claim classes.

A 2025 paper on multi-stage orbital skyhooks reports a modeled two-stage architecture with a combined mass around 3.4 times payload mass. This is a paper-specific design result, not demonstrated hardware or a validation of full orbital-ring infrastructure. Source: https://doi.org/10.1016/j.actaastro.2025.07.018

## 6. Architecture-to-geography workflow

### Stage A — declare service

Specify payload mass/volume, annual cadence, destination orbit/regime, inclination, altitude, delivery accuracy, schedule, cargo/crew class, recovery and reliability service level. Without a defined mission mix there is no meaningful best site.

### Stage B — choose architecture model

Select one architecture family and model version. For rockets, use target orbit and range/vehicle performance. For an elevator, use equatorial tether/counterweight geometry. For ORS/PORS, use the declared ring plane and orbital dynamics. For skyhook, use orbit, tether rotation and capture/release windows. For launch loop, use the ground endpoint geometry and mass-stream dynamics.

### Stage C — compute terminal/access envelopes

Bind Earth-fixed candidate locations to inertial/orbital states through the versioned frame graph (#98) at a named simulation instant. Preserve epoch, frame, ephemeris, transform-model digest and uncertainties. Use #103's conserved transit semantics for actual payload movement and handoff; never teleport mass from the surface to orbit.

### Stage D — ground constraints

Evaluate geodesy, terrain, geology, seismicity, weather, wind, flooding, coastal processes, population/overflight, maritime/airspace, ecology, land/water rights, jurisdiction, energy, water, communications, manufacturing, port/rail/road logistics, workforce and community access.

### Stage E — lifecycle/system evaluation

Evaluate capex, Earth-launched mass, construction/deployment sequence, operations, energy, maintenance, inspection, repair, end-of-life, failure recovery, debris/collision exposure and supplier concentration. A terminal is not feasible simply because its point lies under an orbital ground track.

## 7. Frozen synthetic corpus

At least 20 deterministic, hand-checkable cases:

1. Earth rotation speed at equator and multiple latitudes.
2. Direct eastward injection inclination versus launch latitude.
3. High-inclination/polar mission where the equatorial location loses its presumed advantage.
4. Same latitude but alternate azimuth/downrange population constraints.
5. Rocket launch service compared to an orbital-transfer service target.
6. Elevator equatorial-anchor positive fixture and off-equator anchor negative control.
7. Elevator counterweight/GEO geometry and tether-length bookkeeping.
8. Tether tensile margin sensitivity to specific strength, density, defect fraction and joint efficiency; all material values synthetic until qualified.
9. ORS with equatorial orbital plane and compatible skyhook terminal envelope.
10. ORS with a non-equatorial orbital plane and a different terminal envelope.
11. Orbital-plane precession/epoch shift requiring recomputation.
12. Partial-ring paired endpoints where independent endpoint optimization yields an invalid pair.
13. Skyhook payload capture/release momentum balance and replenishment cost.
14. Ring/ladder payload transfer mass, energy and impulse conservation.
15. Missed transfer window: payload remains in valid transit state.
16. Control loss or propulsion/power fault with explicit fault-domain response.
17. Tether/segment damage, degraded service and recovery envelope.
18. Collision/debris scenario with probabilities unknown unless a qualified model supplies them.
19. Frame mismatch, stale epoch, incompatible time scale and invalid transform negative controls.
20. Earth-side terminal with excellent orbital geometry but invalid rights, safety, ecology or logistics evidence.
21. Two apparent redundant terminals sharing one grid/tracking/range dependency.
22. Multi-mission service portfolio with different orbit inclinations and seasonal windows.

## 8. Metrics and independent checks

- payload delivered per year and useful service windows by mission/orbit class;
- Earth-launched construction mass per unit of recurring payload service;
- lifecycle cost per delivered kg and per mission, with source-qualified currency/time basis;
- energy per delivered kg, peak power and momentum/energy closure;
- terminal coverage, handoff windows and missed-window consequences;
- maximum structural/tether loads, dynamic margins and recovery time where the model supports them;
- fault-set service capacity and failure-domain diversity;
- collision/debris exposure only with regime/epoch-appropriate evidence;
- Pareto regret/hypervolume gap vs exhaustive search for finite cases;
- sensitivity to uncertain materials, mass, orbit, control and deployment assumptions.

Acceptance targets (proposed): 100% of analytic frame/orbit/energy/momentum fixtures match an independent reference within preregistered tolerances; 100% of invalid-frame, stale-epoch, impossible-load, unconserved-transfer and unsupported-material negative controls fail closed; deterministic replay is byte-stable for fixed inputs; and no design is promoted beyond its evidence tier.

## 9. Baselines

Compare rocket access with documented existing launch services and mission analysis; use analytic two-body and Earth-rotation references for simple geometry; use independently implemented numerical orbital propagation for qualified scenarios; use exhaustive finite search for small architecture combinations; and compare search methods under matched hardware, fidelity, compute budget and objective vectors.

Do not claim a better-than-SOTA orbital solution from a toy synthetic ring. Report model fidelity, uncertainty, validation source, numerical tolerances, compute cost and failure coverage alongside any optimization result.

## 10. Claim ceiling and development gates

V0 is conceptual/synthetic. It does not show that an orbital ring, elevator, skyhook, launch loop or tether can be built. No flight, material, structural, dynamic, collision, regulatory or safety qualification is implied.

Promotion ladder: (1) analytic orbital geometry and conservation fixtures; (2) independent numerical dynamics and perturbation studies; (3) measured subscale materials/actuator/tether experiments; (4) uncrewed orbital technology demonstrations; (5) independently reviewed infrastructure concept studies. Do not skip evidence tiers.

Symtropy owns dynamics, transfer and scenario replay. Sol-Atlas maps evidence-qualified Earth-side terminal envelopes. Symthaea may propose design hypotheses and investigation sequences but cannot certify them. Mycelix preserves provenance/rights/decision records but cannot turn signatures into physical truth.