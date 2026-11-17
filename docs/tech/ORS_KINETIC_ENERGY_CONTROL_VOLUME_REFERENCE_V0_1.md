# ORS kinetic-energy control-volume reference — v0.1

**Status:** bounded analytical/software reference; no physical qualification  
**Tracking:** Symtropy [ORS-MECHANICAL-ENERGY-CONTROL-VOLUME-001 #5884](https://github.com/Luminous-Dynamics/symtropy/issues/5884)  
**Companions:** linear momentum #5872, fixed-origin angular momentum #5877  
**Parent:** ORS-DYNAMIC-SUPPORT-001 #5387

## 1. Why kinetic energy gets its own balance

The linear and angular momentum references establish force/torque bookkeeping but do not establish energy closure. A support interaction can have a perfectly closed action/reaction pair while the associated power flow, dissipation, thermal burden and electrical conversion remain unknown.

This increment is deliberately a kinetic-energy ledger, not a general thermodynamics model.

## 2. Balance and sign convention

For a finite open control volume, with each port's mass-flow magnitude already defined as the amount crossing the boundary per unit time:

~~~text
K_CV = integral_CV(0.5 * rho * |v|^2 dV)

Phi_K =
    sum_out(mdot * 0.5 * |v|^2)
    - sum_in(mdot * 0.5 * |v|^2)

dK_CV/dt + Phi_K =
    P_nonport_external
    + P_boundary_work
    - D_mechanical
~~~

All power terms are watts. The velocity at every port is the absolute inertial velocity in the declared frame. P_nonport_external is signed power from all already-derived non-port external mechanisms included in the chosen mechanical boundary. P_boundary_work is signed work rate across the declared mechanical boundary, excluding the advective kinetic-energy already carried by the mass-flow terms. D_mechanical is nonnegative mechanical energy removed from the kinetic ledger through the declared dissipative mechanisms.

The residual is:

~~~text
R_K = dK_CV/dt + Phi_K
      - P_nonport_external - P_boundary_work + D_mechanical
~~~

A scenario is rejected if the mass balance does not close, any input/derived term is non-finite, the mechanical dissipation is negative, or the power residual exceeds a scale-aware predeclared tolerance.

## 3. Why negative power is not energy recovery

A negative external mechanical power means that, under the declared sign convention, energy is leaving the mechanical control volume through that term. It does not establish where the energy goes. It may become heat, field energy, radiation, stored electrical energy, mechanical energy elsewhere, or untracked energy if the chosen system boundary is incomplete.

Therefore the receipt deliberately has no recovered-electrical-power or efficiency field. Those quantities require a separately specified conversion boundary, measured losses, storage state, power electronics, thermal rejection and an independent electrical energy ledger.

## 4. Frozen corpus and checks

Six scenarios and fourteen ports cover:
- straight-through equal-speed flow;
- a speed-changing stream;
- unsteady mass and kinetic-energy storage;
- independent boundary work and mechanical dissipation;
- a negative non-port external power term;
- multiple inlets and outlets at different speeds.

The evaluator sorts ports by canonical ID before summation, records each port's specific kinetic energy and signed power flow, and emits a deterministic receipt tied to the source commit and both input corpus digests.

## 5. VVUQ and intended-use boundary

Use the NASA-STD-7009B / NASA-HDBK-7009B credibility approach and NIST's risk-based computational-model uncertainty practice. A residual at machine precision in a frozen algebraic fixture is verification of the bookkeeping implementation, not validation of real hardware, force laws or thermal/electrical performance.

For real ORS work, the next tiers must add separately derived and verified terms for potential energy, elastic/strain energy, surface tractions, actuator/electrical power, field energy, thermal storage and heat flux. Each term needs an explicit boundary, frame, sign, unit, model identity and uncertainty. Only then can the mechanical energy balance be coupled to a total-energy balance.

## 6. Claim ceiling

This crate verifies only the kinetic-energy storage and advective kinetic-energy balance for its declared pressureless synthetic corpus. It does not show that an orbital ring has enough power, can recover energy, is thermally viable, can survive a fault, or is feasible or safe.
