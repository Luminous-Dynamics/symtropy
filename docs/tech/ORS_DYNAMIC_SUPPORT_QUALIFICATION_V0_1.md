# ORS Dynamic Support Qualification v0.1

**Status:** research specification; not a physical model or feasibility finding  
**Parent:** [Symtropy #3202 — ORBITAL-ACCESS-001](https://github.com/Luminous-Dynamics/symtropy/issues/3202)  
**Work item:** [Symtropy #5387 — ORS-DYNAMIC-SUPPORT-001](https://github.com/Luminous-Dynamics/symtropy/issues/5387)  
**Related:** [Symthaea #7353](https://github.com/Luminous-Dynamics/symthaea/pull/7353), [Symtropy #5358](https://github.com/Luminous-Dynamics/symtropy/pull/5358), [Symthaea #12319](https://github.com/Luminous-Dynamics/symthaea/issues/12319), [Symthaea #12322](https://github.com/Luminous-Dynamics/symthaea/pull/12322)

## 1. Purpose and claim boundary

The next orbital-ring research increment should qualify a **coupled transient support problem**, not extend the T0 circular-orbit calculator into a system it does not represent.

The bounded research question is:

> Given a declared moving mass stream, support geometry, force-port model, control/actuator envelope and initial state, does a reduced-order coupled model conserve the declared quantities, reproduce independent analytical limits, and correctly expose instability or loss-of-support cases?

A successful software or numerical benchmark would not establish that a Birch-style orbital ring can be built. It would establish only that a precisely stated reduced-order model and its implementation pass their declared checks.

Keep these architecture families typed and separate:

- actively supported orbital ring (ORS);
- partial orbital-ring system (PORS);
- tensile tether / skyhook;
- ground-supported launch loop or space fountain;
- passive rigid or flexible ring structure.

A stability result for a passive idealized ring is not evidence for an actively supported momentum-stream system. A PORS ballistic-geometry result is not an ORS support-dynamics result.

## 2. Why transient support is the next gap

Birch's 1982 Part I discusses electromagnetic drag, lift and associated power as part of the orbital-ring concept. The electromagnetic and thermal boundary therefore cannot be treated as a free, lossless constraint. See *Orbital Ring Systems and Jacob's Ladders — I*, especially §3.4: https://nss.org/wp-content/uploads/Orbital-Rings.pdf

Birch's 1983 Part II describes a specific failure mechanism: if a skyhook load is suddenly reduced, an orbital ring may overshoot and miss a subsequent support, potentially leading to breakup. It proposes independently operating riders and spring/damped coupling as a possible mitigation. That is a historical proposed mitigation, not a demonstrated safety result. See *Orbital Ring Systems and Jacob's Ladders — II*, §5.6: https://www.orionsarm.com/fm_store/OrbitalRings-II.pdf

Accordingly, equilibrium-only analysis is insufficient. At minimum, the research campaign must expose load steps, support loss, control delay, actuator saturation, rider coupling failure, stream discontinuity and common-cause faults.

## 3. T0 reference boundary

The existing Symthaea T0 work supplies useful ideal circular-orbit references and a bounded steady-stream limiting expression. For a steady stream with line mass density \(\lambda\), inertial speed \(u\), and fixed circular path radius \(R\), the normal resultant per unit path length is:

\[
q_n = \frac{\lambda u^2}{R}.
\]

This is a kinematic/control-volume limit under its declared assumptions. It is **not** an electromagnetic constitutive law, a support-actuator law, an equilibrium solution for a moving support, or a ring-stability result.

Do not infer actuator force, magnetic field strength, cryogenic load, support stiffness, or stability margin from this expression.

## 4. T1 reduced-order model boundary

The first model should contain the minimum coupled states needed to test the missing semantics:

1. A finite stream control volume with declared path coordinate, inertial frame, mass density, stream velocity, curvature, inflow/outflow and source/sink terms.
2. One or more support/rider nodes with position, velocity, effective inertia, geometry, constraints and explicit load paths.
3. A named force-port law between stream and support. Every law must carry version, parameters, units, calibration/evidence class, applicability envelope and excluded terms.
4. Actuator and controller state: force/current bounds, slew limits, control latency, power availability, shutdown mode and saturation semantics.
5. Optional rider/coupler state: spacing, stiffness, damping, travel, saturation, independence and shared-cause dependencies.
6. Explicit environment and initial-condition uncertainty, including the provenance for any claimed initial equilibrium.

For the first software-only coupling tests, a bounded synthetic linear force-port surrogate may be used, for example:

\[
F_s = -k x - c\dot{x}.
\]

It must be labelled **SYNTHETIC / NOT_PHYSICAL** in the input, output and evidence record. It tests interface semantics, integration, latency and fault handling only. It must never be presented as a magnetic levitation, induction, superconducting-bearing or electromagnetic support model.

The physical electromagnetic model is a separate admission gate. Before using it for an engineering claim, identify the actual conductor/field/geometry/current/material model and its calibration domain. Where the field is represented dynamically, the accounting must include field momentum and field-energy storage/transport, not just mechanical forces on the two bodies.

## 5. Conservation and control-volume requirements

Every scenario must declare the stream, support, field, electrical and thermal control volumes and their interfaces. A conservation statement is meaningful only for the declared boundary.

For a one-dimensional stream with line density \(\lambda(s,t)\), inertial velocity vector \(\mathbf v(s,t)\), and transport speed \(u(s,t)\) along the declared path coordinate, the mass balance takes the form:

\[
\frac{\partial\lambda}{\partial t}
+\frac{\partial(\lambda u)}{\partial s}
= q_m,
\]

where \(q_m\) is a declared mass source per unit length per unit time. The corresponding momentum balance must retain boundary momentum flux and the declared external interaction terms. In integral form, for a fixed control volume:

\[
\frac{d}{dt}\int_{CV}\lambda\mathbf v\,ds
+\left[\lambda u\mathbf v\right]_{\partial CV}
= \int_{CV}\mathbf f_{ext}\,ds
+ \text{declared source momentum}.
\]

For a moving control-volume boundary, use the appropriate relative boundary velocity in the flux terms. The derivation note and implementation must define the sign convention and boundary motion explicitly; the compact equation above is not permission to omit those terms.

The model must keep separate, rather than hiding in a balancing residual:

- stream mass and mass flux;
- stream linear and angular momentum;
- support/rider linear and angular momentum;
- actuator work and electrical input;
- mechanical kinetic and elastic energy;
- field energy and field momentum where the field is in scope;
- Poynting flux and Maxwell-stress transfer at declared electromagnetic boundaries;
- resistive, eddy-current and other dissipative heat;
- cryogenic/refrigeration input and rejected heat where those systems are included;
- external work, environmental exchange and any unmodelled residual.

An equal-and-opposite mechanical interface-force check is valid only when the field/control-volume momentum exchange has been included or shown to be negligible within a justified approximation. Do not silently attribute momentum carried or stored by the field to the stream or support.

Every unexplained residual remains an explicit failure or unknown. It must not be absorbed into an arbitrary “loss” term.

## 6. Scenario matrix

The first frozen corpus should include at least these classes:

| ID family | Scenario | Required observable |
|---|---|---|
| EQ | Nominal equilibrium and zero perturbation | State invariance; force/moment residual |
| LIN | Small perturbation about an admitted equilibrium | Independent linearized poles, damping and mode response |
| STEP | Support load step and finite ramp | Peak displacement, velocity, force, impulse |
| RELEASE | Sudden payload / skyhook load release | Overshoot, missed-handoff and time-to-limit |
| RIDER | Rider loss or changed spacing/stiffness/damping | Coupling response and remaining support authority |
| SAT | Actuator saturation and rate limit | Saturation duration, residual error and recovery |
| DELAY | Control and sensing latency | Stability margin and delay sensitivity |
| POWER | Power interruption and controlled shutdown | Safe-state transition and stored-energy disposition |
| STREAM | Stream mass-flow / speed reduction or segment loss | Momentum-flux and transient response |
| COIL | Lift-coil fault | Distinct force loss and electrical/thermal disposition |
| QUENCH | Superconducting quench | Distinct quench dynamics, field-energy and thermal path |
| ASYM | Counter-rotating or paired-stream imbalance | Net momentum and angular-momentum response |
| COMMON | Common-cause loss of redundant supports or power | Independence failure and degraded-state reachability |
| BAD | Invalid frame, stale epoch, impossible load, missing law or incomplete boundary | 100% fail-closed rejection |

Lift-coil interruption and superconducting quench are different fault families; do not merge them into a generic “magnet failure.” A rider loss, a load release and a stream-segment loss must also remain distinguishable.

## 7. Verification ladder and industry comparisons

Use independent, fit-for-purpose tools rather than building a competing universal astrodynamics or FEM stack.

### Orbital mechanics

Use Orekit and NASA GMAT as independent trajectory/frames/force-model comparators where the required model profiles overlap. GMAT is an established open-source mission-design and navigation system used across LEO, lunar, libration-point and deep-space analysis: https://gmat.gsfc.nasa.gov/

Basilisk provides an additional spacecraft-dynamics and control comparison path, including module tests, integrated scenarios and conservation-oriented checks: https://github.com/AVSLab/basilisk

Compare only matched cases. Freeze epoch/time scale, frame realization, Earth orientation, gravity model, ephemeris, atmosphere, force models, integrator, tolerances and output sampling. Report model differences; do not call tool agreement physical validation.

### Structures and dynamics

Build on the existing Symtropy analytical rotating-ring/spoke reference, sparse modal work, FEM adapter qualification and as-built truss acceptance. Do not start a second general-purpose structural solver.

NASA JSC 65829 Rev B requires anticipated static and dynamic load events over the hardware lifecycle to be assessed and discusses verification of dynamic models against modal survey tests with appropriate boundary conditions. Use this as a minimum qualification mindset: analytic verification → numerical convergence → independent solver comparison → modal / component test correlation → integrated hardware-in-the-loop evidence. Standard reference: https://standards.nasa.gov/standard/jsc/jsc-65829

A converged FEM result or a pair of agreeing solvers is not, by itself, physical validation.

### Required verification tiers

- **T0 — analytic references:** current circular-orbit and steady-stream limits, plus closed-form limiting cases.
- **T1 — reduced-order transient:** one stream control volume and one or more support/rider nodes, initially using synthetic force-port surrogates.
- **T2 — multi-support segment:** spatially distributed stream/support and rider interactions, admitted force law, propagation delay and fault containment.
- **T3 — coupled multiphysics:** electromagnetic field / circuit / mechanical / structural / thermal model with qualified backends and verified coupling.
- **T4 — experimental correlation:** component and subsystem tests, modal survey, calibrated sensors, hardware-in-the-loop, fault injection and quantified model discrepancy.
- **T5 — system demonstration:** only after independent design, safety, range, operations, environmental and regulatory reviews. This document does not define or authorize such a demonstration.

## 8. Metrics: separate, uncertainty-aware, decision-relevant

Report at least:

- equilibrium force and moment residuals (dimensional and normalized);
- mass, linear-momentum and angular-momentum closure;
- energy ledger closure, including field and thermal terms when in scope;
- linearized eigenvalues, mode shapes, damping and uncertainty;
- peak support displacement, velocity, force and interface impulse;
- maximum stream/path deviation and minimum support/handoff clearance;
- time-to-limit, time-to-safe-state, recovery time and remaining actuator authority;
- force/current/power peaks, integrated electrical energy and heat rejection;
- sensitivity to latency, saturation, rider spacing, damping, stream state and initial-condition uncertainty;
- numerical convergence and solver-to-solver discrepancy;
- fault-containment coverage, common-cause exposure and negative-control rejection rate.

Keep physical uncertainty, numerical error, model-form discrepancy and evidence maturity separate. Do not collapse them into a single “stability score.”

## 9. Symthaea / Symtropy / Sol-Atlas responsibilities

- **Symthaea:** propose candidate configurations, identify decision-sensitive unknowns, select the next admissible analysis or experiment, and preserve competing hypotheses and model-discrepancy records. It cannot dispatch unqualified physics or certify its own predictions.
- **Symtropy:** admit the exact typed request, execute only allowlisted model profiles, replay deterministically, run independent oracles and return a receipt with explicit unsupported physics and claim ceiling.
- **Sol-Atlas:** project read-only terminal, route, terrain, environment, community and infrastructure evidence. It is not an orbital mechanics or site-approval authority.
- **Mycelix / evidence infrastructure:** preserve provenance, rights and authorization as applicable. A signed record does not make a physical claim true.

Use immutable, versioned handoffs and exact input/output identities. Do not copy or fork another project's solver, evidence registry or orbital propagator.

## 10. Acceptance and promotion rules

A T1 implementation is ready for independent review only when:

1. All positive cases have independently derived expected behavior.
2. All negative/adversarial cases fail closed with stable reason codes.
3. The same frozen inputs produce byte-identical canonical outputs where determinism is claimed.
4. Time-step / integration-tolerance refinement is reported and meets preregistered bounds.
5. Linearized and time-domain results agree within declared numerical tolerances.
6. Conservation residuals are dimensionally and physically interpretable; no unexplained residual is re-labelled as dissipation.
7. Every result binds model, code, toolchain, configuration, input, output and verifier identities.
8. The synthetic force-port surrogate is visibly and machine-readably prevented from promotion to a physical magnetic-support claim.

No claim of orbital-ring feasibility, magnetic-support viability, safe failure behavior, construction readiness, crew safety or flight readiness may be made from T0/T1 evidence alone.

## 11. Proposed research milestones

1. **Derivation freeze:** independent review of control-volume boundaries, momentum-flux terms and the field/energy accounting boundary.
2. **T1 contract and corpus:** schema, synthetic fixtures, negative controls and claim ceilings.
3. **Independent evaluator:** separate implementation for reduced-order reference cases and linearized dynamics.
4. **Transient fault campaign:** load step, support loss, actuator delay/saturation, power interruption, coil interruption and quench as distinct cases.
5. **Solver interoperability:** compare suitable orbital / structural submodels to GMAT, Orekit, Basilisk and qualified FEM backends only where their domains overlap.
6. **Physical model admission:** add an electromagnetic force law only with traceable derivation, calibration and applicability; add structural/thermal fidelity only through qualified existing owners.
7. **Test correlation plan:** define coupon, component, modal and hardware-in-loop measurements before any higher-tier claim.

## References

- Birch, P. (1982), “Orbital Ring Systems and Jacob's Ladders — I,” *Journal of the British Interplanetary Society* 35, 475–497. https://nss.org/wp-content/uploads/Orbital-Rings.pdf
- Birch, P. (1983), “Orbital Ring Systems and Jacob's Ladders — II,” *Journal of the British Interplanetary Society* 36, 115–128. https://www.orionsarm.com/fm_store/OrbitalRings-II.pdf
- NASA Goddard, General Mission Analysis Tool (GMAT), R2026. https://gmat.gsfc.nasa.gov/
- NASA Johnson Space Center, JSC 65829 Rev B, *Loads and Structural Dynamics Requirements for Spaceflight Hardware*. https://standards.nasa.gov/standard/jsc/jsc-65829
- AVS Lab, Basilisk spacecraft simulation framework. https://github.com/AVSLab/basilisk
- Orekit, open-source space flight dynamics library. https://www.orekit.org/

---

**Claim ceiling:** This document is a research and qualification specification. It is not an executable physics result, an independently verified force law, a design approval, or evidence that an orbital ring can be built.
