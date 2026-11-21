# Conservative Plasma–Neutral Exchange Contract for EHD (2026)

**Contract:** EHD-PLASMA-NEUTRAL-EXCHANGE-001  
**Status:** protocol specification; implementation and solver validation pending  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_plasma_neutral_exchange_contract_v0_1.json](../../data/research/ehd_plasma_neutral_exchange_contract_v0_1.json)

## Why this is the next technical frontier

The 2026 multistage ducted-thruster study has already demonstrated a useful automated workflow: CMA-ES geometry search, a dedicated plasma model, and 2D incompressible Navier–Stokes. It reports approximately 6× initial thrust density but around 0.5× the initial thrust-to-power ratio. Its result is numerical, not a validated physical improvement. Its model is stationary with zero freestream, uses constant air density/kinematic viscosity, restricts collector shapes to a small NACA parameterisation, estimates thrust from pressure differences and electrode pressure drag, and omits friction drag on electrodes and duct. The article says experiments are planned and data are available by request. [Shibata et al., AIP Advances 16, 015101 (2026)](https://doi.org/10.1063/5.0300448).

A separate 2025 paper establishes mathematical properties—charge conservation, an unconditional discrete energy law and convergence—for a fully discrete three-dimensional EHD finite-element method. Its governing physical system is injected charge in an insulating non-polar dielectric liquid. It is valuable as a numerical-analysis precedent, but not an atmospheric corona-air validation case. [Zhang, Zhang & Huangfu, J. Math. Anal. Appl. 550 (2025), 129592](https://doi.org/10.1016/j.jmaa.2025.129592).

The difficult engineering gap is therefore not another optimiser. It is ensuring a discharge solver and a neutral-flow solver exchange **exactly the right physical quantities, exactly once**, with known conservation errors and a reproducible system boundary.

## 1. Force-transfer methods must be explicit and exclusive

For a declared species-resolved ion-neutral drift/friction closure, one possible momentum-transfer term is

\[
\mathbf R_{s\rightarrow n}
  = m_s n_s \nu_{sn}(\mathbf u_s-\mathbf u_n),
\]

where species mass, density, momentum-transfer frequency, species velocity and neutral velocity are all tied to the same declared collision model and units. The neutral momentum equation receives the sum of the species momentum-transfer terms. The charged-species momentum balance receives the exact equal-and-opposite reaction.

The electric force density

\[
\mathbf f_{E,s}=q_s n_s\mathbf E
\]

acts first on the charged phase. It is not automatically a second source to inject into the neutral Navier–Stokes equation when explicit ion-neutral momentum transfer is already being applied. Doing both for the same charge-to-neutral exchange generally double-counts momentum.

The contract therefore defines two alternative closures:

| Method | Neutral source | Required condition |
|---|---|---|
| Explicit species collision drag | Sum of declared ion-neutral momentum-transfer operators | The charged-phase reaction is accounted exactly equal and opposite |
| Reduced Coulomb deposition | A qualified reduced \(\rho_c\mathbf E\) term | Charged momentum storage, flux, pressure/stress and boundary terms are shown negligible or included in the reduction |

Only one method may represent a particular physical momentum-transfer path in a run. The reduced method is not an escape hatch: it requires an explicit derivation, omitted-term bounds, and an independent validation receipt.

## 2. Exchange must be conservative across solver boundaries

Plasma and neutral-flow solvers may use different meshes, cell layouts and time steps. Nodal interpolation of a force-density field is not, by itself, a guarantee that its integrated impulse is preserved.

The exchange object should carry the space-time integrated impulse over matched control volumes:

\[
\mathbf I_{s\rightarrow n}
= \int_{t_0}^{t_1}\int_{\Omega}
  \mathbf R_{s\rightarrow n}\,dV\,dt.
\]

The mapping operator must record its source and destination mesh identities, time-grid identities, quadrature, operator version, mapped impulse and residual. Its acceptance test is integral preservation to a case-specific, predeclared tolerance. The charged-side reaction uses the same integrated exchange, with the opposite sign.

This creates a strong separation between:
- solving the local plasma and neutral equations;
- mapping physical exchange between discretisations;
- qualifying the coupled run against its charge, momentum and energy ledgers.

## 3. Three independent ledgers

### Charge

Audit the complete charged-species system:

\[
\frac{\partial \rho_c}{\partial t}
+\nabla\cdot\mathbf J_c=S_q.
\]

For a complete species set, charge-conserving chemistry must preserve net charge. The integrated ledger includes initial and final charge, boundary current, net chemical charge source, residual and uncertainty.

### Momentum

Track charged momentum, neutral momentum, boundary momentum flux, electrode/support impulse, electromagnetic field-stress contribution or a justified neglect bound, interphase exchange and uncertainty.

Do not count the same interaction once as a Coulomb body force and again as ion-neutral drag. Do not combine a force-balance reading and a flow-derived momentum-flux result as additive thrust; they are independent observables for cross-checking.

### Energy

Track measured terminal electrical input, field-energy change, charged and neutral kinetic energy, Joule/collisional heating, chemical/excitation/ionisation energy, electrode/wall heat, radiation or a declared bound, outward energy flux, residual and uncertainty.

A missing term is null with a reason, not zero. Wall-plug power must remain distinct from nominal supply voltage times a current sampled elsewhere in the circuit.

## 4. Validation ladder before any coupled-thrust claim

1. Equal-and-opposite impulse for every exchange contribution.
2. Uniform-drift analytic transfer case.
3. Zero-charge/zero-field limit.
4. Polarity reversal with predicted sign.
5. Global charge and boundary-current closure.
6. Mesh partition / conservative projection invariance.
7. Time-grid exchange impulse consistency.
8. Nonnegative collisional dissipation for the declared drag model.
9. Agreement of direct force and control-volume momentum-flux observables within combined uncertainty.
10. A negative control that rejects explicit drag plus direct Coulomb deposition for the same physical transfer.

There is no universal residual threshold for all gases, geometries, solver families and experimental rigs. Tolerances must be predeclared for each case, reflect discretisation and instrument uncertainty, and remain separate by ledger.

## 5. What to benchmark against—and how to move beyond it

### Automated topology optimisation

The Shibata et al. 2026 workflow is a direct incumbent for computational topology search. It reports a Pareto front from a 2D stationary workflow with seven individuals per generation and four latent design variables. Its objective combines thrust per span (N/m) and thrust-to-power; its reported ~6× thrust-density gain is relative to its initial design plan, with a roughly half-sized thrust-to-power ratio. The paper's thrust estimate ignores friction drag, and no physical validation is presented in the paper.

Do not compare its N/m thrust-per-span to an N/m² area-normalised thrust density without a declared conversion and geometry.

### Commercial multiphysics

COMSOL exposes fluid, electrostatic, species transport, plasma and nonisothermal plasma-flow capabilities, while documenting limitations of individual interfaces. Its simplified Corona Discharge interface, for example, omits the ionisation layer and electron dynamics. A fair benchmark records exact interface, equations, chemistry, dimensions, circuit, mesh and flow coupling—not just the vendor name. [COMSOL Plasma Modeling](https://doc.comsol.com/6.3/doc/com.comsol.help.plasma/plasma_introduction.02.02.html).

### Numerical-analysis precedent

The 2025 three-dimensional dielectric-liquid EHD finite-element work is a strong target for numerical properties: charge conservation, discrete energy stability and convergence. It is not a physical validation substitute for a gas-corona thruster; the chemistry, charge injection and neutral-flow regime differ.

### Luminous differentiation target

A credible competitive advantage would be a workflow in which:
- every solver handoff is conservative and replayable;
- mesh/time exchange residuals are reported;
- each force-transfer mechanism is exclusive and auditable;
- charge, momentum and energy close independently;
- uncertainty and unsupported regimes trigger abstention;
- geometry optimisation is evaluated on full drag, forward-flow, thermal and power boundaries;
- top candidates are tested on held-out geometry families and an independent experimental session.

## 6. Immediate implementation sequence

1. Implement the machine-readable contract validator in the offline Rust audit.
2. Add analytic and mutation tests before writing a solver adapter.
3. Bind Merlino2D as a discharge-only upstream component and a separate neutral-flow solver as a downstream component; do not pretend Merlino2D already closes thrust.
4. Qualify the exchange adapter on analytic manufactured cases.
5. Reproduce a source-pinned wind-tunnel flow/force cohort with the actual frame/electrode drag terms.
6. Only then connect the coupled solver to Symthaea topology and multi-fidelity experiment selection.
7. Compare with the 2026 CMA-ES/ANEMONE-lite baseline under matched geometry, observable, drag and power definitions.

## 7. Claim ceiling

This document specifies a contract. It does not claim that the coupling interface, a new plasma/neutral solver, analytic tests, physical measurements, thrust validation, or SOTA superiority have been implemented or executed.
