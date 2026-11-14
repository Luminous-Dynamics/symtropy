# Celestial Dynamics Benchmark Ladder (2026)

Status: **Proposed reference protocol**. The current `symtropy-celestial` crate is a Newtonian point-mass reference integrator using direct pair forces and velocity-Verlet. This ladder does not qualify it as an ephemeris, a planetary-science package, a relativistic solver, or a cosmological N-body code.

## Paradigm shift: test claims, not visual plausibility

A simulation is not validated because its orbit looks plausible or because one run conserves one quantity. Keep three questions separate:

1. **Numerical verification:** did this implementation solve the declared equations to the requested tolerance?
2. **Model validation:** are those equations and assumptions appropriate for the intended physical system?
3. **Operational qualification:** is the tool reliable for the particular decision, domain, runtime, and uncertainty budget?

A pass at one layer must never imply a pass at the next. Report a vector of metrics—convergence, invariant drift, failure handling, runtime and domain coverage—rather than collapsing them into an opaque single score.

## Frozen run contract

Every benchmark receipt should record:

- exact repository and full commit SHA;
- benchmark protocol version and case identifier;
- integrator, step size or adaptive tolerances, step count and end time;
- G, all masses/initial positions/velocities and units;
- softening length (zero for the unsoftened Newtonian fixtures);
- input fixture digest and expected-claim/tolerance digest;
- toolchain, target OS/architecture, CPU/GPU if used, build profile and command;
- initial/final energy, linear momentum, angular momentum and center-of-mass diagnostics where applicable;
- maximum and final relative errors separately, plus elapsed time and number of force evaluations when measured;
- pass/fail/inconclusive and an explanation of the claim ceiling.

A benchmark without the actual run receipt is a **target**, not a result. Never fill missing timings or reference outputs with estimates.

## Qualification levels

| Level | Question and fixture | Acceptance target |
|---|---|---|
| N0 — domain and failure boundaries | Duplicate IDs, invalid masses/time steps, non-finite state, singular zero-softening pairs, and numerical overflow | Invalid cases return declared errors; a rejected step leaves the whole state and step/time counters unchanged |
| N1 — analytic two-body | Equal-mass circular binary, then an eccentric Kepler orbit with e = 0.6; compare one-period closure against the analytic period | Current code assertions: circular case (1,000 steps) relative position closure < 1.1e-4, relative energy drift < 1e-10; eccentric case (1,024 → 2,048 steps) finer closure < 0.3× coarse and fine-run relative energy drift < 2e-9 |
| N2 — numerical convergence | Repeat fixed initial conditions with dt, dt/2, dt/4; sweep orbital eccentricity, not just one circular case | Report empirical order from errors at all levels. Velocity-Verlet should show second-order convergence while temporal and floating-point errors remain dominant; do not hard-code a claimed order without measured ratios |
| N3 — long-run invariants | Long isolated integrations with zero softening, then separately with a fixed positive Plummer softening and its matching potential | Freeze initial values; report max and final energy error, momentum/angular-momentum drift, center-of-mass drift and step count. Treat softened and unsoftened runs as different force models |
| N4 — independent solver oracle | Run the exact same Newtonian setup in an independently maintained integrator, first for two-body, then for non-chaotic small-N cases | Freeze versions, methods, tolerances, units and initial bytes. Agreement should be checked over a stated time horizon and against invariants—not inferred from matching a plot |
| N5 — chaotic three-body | Frozen initial states and seeds with close approaches/encounters; compare trajectories only over a declared horizon | Beyond sensitive-dependence horizons, trajectory divergence alone is not a verdict. Compare energy/angular-momentum envelopes, encounter counts, escape/ejection outcomes, invariant distributions and sensitivity under timestep refinement |
| N6 — scaling and cost | N = 2, 4, 8, 16, 32, 64 or larger only after memory/runtime are safe; benchmark force evaluation separately from setup and reporting | Report median and spread across repeated runs, N² force-work expectation, allocation strategy and error at each point. Publish no speed-up claim until measured on a named machine against a frozen baseline |
| N7 — domain extension | Planetary ephemeris, tides/collisions, relativistic corrections or cosmological structure formation | Each extension requires its own equations, fixtures, independent reference and validity domain. The current Newtonian point-mass crate does not qualify these uses |

### Current in-tree evidence versus future gates

The crate currently includes unit-test assertions for N0 and N1, circular-orbit refinement, and short-run invariants. The eccentric-orbit test is a newly added gate and must pass on the exact CI head before it is described as passing. N4–N7 remain future work; no REBOUND comparison or scaling measurements have been performed by this change.

## Independent oracle choice

[REBOUND](https://rebound.hanno-rein.de/) is a useful comparison target because its maintained documentation describes several independent numerical approaches, including high-accuracy adaptive IAS15 and specialized symplectic/hybrid integrators such as WHFast, MERCURIUS and TRACE ([integrator reference](https://rebound.hanno-rein.de/integrators/)). This does not make REBOUND an infallible oracle: the exact integrator, version, force model, settings and output sampling must be pinned, and the reference must use equivalent assumptions.

For the first external comparison, use its high-accuracy method as an oracle for the same unsoftened Newtonian two-body initial conditions. Only after matching analytic results and declared error envelopes should small-N and chaotic fixtures be added. For chaotic runs, freeze the comparison horizon and distinguish finite-time trajectory agreement from statistics of longer runs.

## How this composes with the science stack

- **Symtropy:** evolves the explicitly declared physical state under the declared equations.
- **Symthaea:** may propose hypotheses, predict expected measurements or design the next experiment; its confidence is not the solver's validation.
- **Mycelix:** can bind the claim to exact inputs, artifact hashes, run receipts, independent verifier identity and supersession/retraction history.
- **External references and observations:** remain independent. Agreement with a second program verifies implementation against that program's assumptions; observational validation is a separate claim.

## Exit gates before making broader claims

1. All analytic tests pass at the exact commit on Linux, macOS and Windows.
2. Timestep-refinement ratios are published from actual runs, not only asserted in source.
3. An independent solver comparison has a frozen fixture, reference version and machine-readable discrepancy report.
4. Negative controls prove mutation of initial bytes, (G), softening, units or timestep invalidates/replaces the receipt.
5. Every public report distinguishes **not run**, **failed**, **passed numerical check**, **independently reproduced**, and **validated for a stated domain**.

The goal is not to win a leaderboard with a plausible animation. It is to make each physical claim reproducible, falsifiable and honest about the limits of the model.
