# ORS finite control-volume momentum-flux reference — v0.1

**Status:** bounded analytical/software reference; no physical qualification  
**Tracking:** Symtropy [ORS-MOMENTUM-CONTROL-VOLUME-001 #5872](https://github.com/Luminous-Dynamics/symtropy/issues/5872)  
**Parent:** [ORS-DYNAMIC-SUPPORT-001 #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)

## 1. Why this increment exists

The existing orbital-ring work has distinct references for a fixed inertial circular stream, Birch's skyhook turn approximation, PORS ballistic endpoint geometry, and a synthetic two-body spring/damper transient. None should be silently reused as a general open-system momentum equation.

For a moving stream, material can cross the boundary of the chosen control volume. A material-element equation without boundary fluxes is not the same as the general control-volume balance. The present reference isolates the exact algebraic bookkeeping needed before any orbital-ring support force is interpreted.

## 2. Balance and sign convention

Choose one declared inertial frame and one control-volume boundary. Every mass-flow magnitude is non-negative. Each port separately declares whether it is an inflow or an outflow.

The mass balance is:

~~~text
dM_CV/dt = sum_in(mdot) - sum_out(mdot)
~~~

The advective linear-momentum balance is:

~~~text
F_required_on_stream =
    d/dt(integral_CV(rho * v dV))
    + sum_out(mdot * v) - sum_in(mdot * v)
~~~

The implementation names the flux difference explicitly. It then isolates the support interaction:

~~~text
F_support_on_stream =
    F_required_on_stream - F_other_external_on_stream

F_stream_on_support = -F_support_on_stream
~~~

The other-external-force vector must represent the already-derived sum of every non-support external force admitted inside the declared boundary. This reference does not infer gravity, pressure, field or structural forces.

Mass-balance mismatch is a hard failure. Non-finite inputs or derived results, negative mass-flow magnitudes, duplicate port IDs, unknown case IDs and invalid direction semantics also fail closed.

## 3. Exact turn geometry

For a steady single-stream turn with equal inlet/outlet mass-flow magnitude mdot and equal speed V, the vector momentum flux is:

~~~text
DeltaPdot = mdot * (V_out - V_in)
~~~

If the speed directions differ by theta and their magnitudes are equal:

~~~text
|DeltaPdot| = mdot * 2 * V * sin(theta / 2)
~~~

With mdot = lambda * V:

~~~text
|DeltaPdot| = lambda * V^2 * 2 * sin(theta / 2)
~~~

For small theta this tends to lambda * V^2 * theta, matching the first-order momentum-rate form used in Birch's skyhook model. The executable does not substitute the approximation at finite angles. It sums the actual three-dimensional inlet and outlet momentum vectors.

This is not a derivation of Birch's orbital conic or his support architecture. It is the exact vector balance for the declared pressureless mass-stream control volume.

## 4. Frame-covariance checks

The frozen tests exercise two important properties.

**Orthogonal rotation.** Rotate every port velocity, momentum-accumulation vector and other-force vector by the same orthogonal transform. The outputs must rotate by that transform; scalar mass residuals and force norms must remain unchanged.

**Galilean boost for an unsteady control volume.** For a coordinate frame moving at constant velocity u, port velocities transform as v' = v - u. The momentum-accumulation derivative transforms as:

~~~text
dP'_CV/dt = dP_CV/dt - u * dM_CV/dt
~~~

The advective flux changes by the opposite amount because the mass balance is enforced. Therefore the required net force is invariant under this Newtonian frame change. The metamorphic test verifies this identity with non-zero mass accumulation; a steady-only test would not expose the mass-accumulation term.

These are algebraic covariance checks under Newtonian assumptions. They do not qualify a relativistic model or prove that the physical scenario is correctly framed.

## 5. Frozen corpus and metrics

The six scenarios contain fourteen ports. Results are kept as vectors and dimensional residuals. The suite reports:

- mass flow in/out and mass-balance residual (kg/s);
- advective momentum-flux vector (N);
- required external resultant (N);
- support-on-stream force and equal/opposite stream-on-support reaction (N);
- reaction-closure residual (N);
- frame-covariance residuals and finite-angle/small-angle discrepancy in tests.

No composite “physics score” is used. Failed and negative cases remain part of the suite denominator.

## 6. VVUQ and industry comparison

NASA-STD-7009B makes intended-use credibility and acceptance criteria explicit in model/simulation lifecycles. NASA's February 2026 implementation handbook gives additional practice guidance. The present reference follows that philosophy at a small mathematical tier: declare the use, boundary, assumptions, exclusions, independent analytic cases and evidence claim ceiling.

The generic astrodynamics field already has mature benchmarking. Brahe's published comparison reports 32 tasks across eight modules, compares API-compatible subsets of Orekit 13.1.5, GMAT R2026a, Basilisk and Nyx/ANISE, separates timing from accuracy, and uses 200 independent initial conditions for distributional accuracy summaries. We should reuse that benchmark discipline for generic orbital frame/time/propagation tasks rather than try to beat those libraries with an unmatched microbenchmark.

Our differentiating benchmark should be the missing ORS-specific problem: exact multi-port advective momentum accounting, unsteady mass storage, Newtonian frame covariance, support-reaction sign correctness, and later coupling to independently qualified structural/electromagnetic backends. There is no meaningful SOTA claim until these tasks are compared against independent analytical oracles and, later, measured physical data.

Sources:
- [NASA-STD-7009B](https://standards.nasa.gov/node/263)
- [NASA-HDBK-7009B, published 3 February 2026](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009)
- [Brahe astrodynamics benchmark suite](https://docs.brahe.space/latest/about/benchmarks.html)

## 7. Promotion boundary and next work

This reference does not contain pressure/stress surface traction, field momentum, material mechanics, structural response, angular-momentum flux or energy flux. Those terms are not silently zero in a physical orbital-ring system. They must be separately derived, versioned and independently checked before a support force computed from a populated scenario is used as a physical design load.

Next increments should be separate:
1. Add an independently derived angular-momentum flux reference and declared moment arms.
2. Add a separate mechanical-energy/power ledger with boundary work and energy flux; do not infer electrical recovery from negative mechanical port work.
3. Define the real support/rider topology and its force law only from source-backed or measured constitutive evidence.
4. Compare against independent solver products on matched boundaries and force models.
5. Move toward T1 transient coupling only after a complete equilibrium and all conservation boundaries are specified.

A passing software result does not establish a feasible or safe orbital ring.
